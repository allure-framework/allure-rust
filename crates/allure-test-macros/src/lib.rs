//! Procedural macros used by `allure-cargotest`.
//!
//! Most users import these macros through `allure_cargotest`, which re-exports them together with
//! the cargo-test runtime integration.

#![deny(missing_docs)]

use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

mod doc_comments;

struct ShouldPanicConfig {
    enabled: bool,
    expected: Option<String>,
}

#[derive(Debug)]
struct AttrArgs {
    name: Option<String>,
    id: Option<String>,
    doc: bool,
}

#[derive(Debug)]
struct StepAttrArgs {
    name: Option<String>,
}

#[derive(Debug)]
struct ExplicitReturnType {
    ty: String,
    arrow_start: usize,
    end: usize,
    kind: ReturnTypeKind,
}

#[derive(Debug, PartialEq, Eq)]
enum ReturnTypeKind {
    Concrete,
    Opaque,
    Never,
}

#[proc_macro_attribute]
/// Wraps a helper function body in an Allure step.
///
/// The optional `name = "..."` argument overrides the step name. When omitted, the Rust function
/// name is used as the step name.
pub fn step(args: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = match parse_step_args(args) {
        Ok(attrs) => attrs,
        Err(err) => return compile_error(err),
    };

    transform_step_fn(attrs, input)
}

#[proc_macro_attribute]
/// Wraps a `cargo test` test function in an Allure test lifecycle.
///
/// Supports optional `name = "..."` for the display name, `id = "..."` for the Allure ID label,
/// and `doc = false` to disable doc-comment descriptions.
/// Rust doc comments on the test function are used as the default markdown description unless
/// `doc = false` is set.
pub fn allure_test(args: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = match parse_args(args) {
        Ok(attrs) => attrs,
        Err(err) => return compile_error(err),
    };

    transform_fn(attrs, input)
}

#[proc_macro_attribute]
/// Rewrites standard Rust assertions in a function body into Allure assertion log steps.
///
/// This is useful for helper functions that are not already annotated with `#[allure_test]` or
/// `#[step]`.
pub fn log_asserts(args: TokenStream, input: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return compile_error("#[log_asserts] does not accept arguments");
    }

    transform_log_asserts_fn(input)
}

fn parse_step_args(args: TokenStream) -> Result<StepAttrArgs, &'static str> {
    let attrs = parse_kv_args(args, &["name"])?;
    Ok(StepAttrArgs { name: attrs.name })
}

fn parse_args(args: TokenStream) -> Result<AttrArgs, &'static str> {
    parse_kv_args(args, &["name", "id", "doc"])
}

fn parse_kv_args(args: TokenStream, allowed_keys: &[&str]) -> Result<AttrArgs, &'static str> {
    let tokens: Vec<TokenTree> = args.into_iter().collect();
    if tokens.is_empty() {
        return Ok(AttrArgs {
            name: None,
            id: None,
            doc: true,
        });
    }

    let mut idx = 0;
    let mut parsed = AttrArgs {
        name: None,
        id: None,
        doc: true,
    };
    let mut doc_seen = false;

    while idx < tokens.len() {
        let key = match &tokens[idx] {
            TokenTree::Ident(id) => id.to_string(),
            _ => return Err("unsupported attribute arguments, expected key = \"value\""),
        };
        idx += 1;

        match tokens.get(idx) {
            Some(TokenTree::Punct(eq)) if eq.as_char() == '=' => idx += 1,
            _ => return Err("unsupported attribute arguments, expected key = \"value\""),
        }

        match key.as_str() {
            "name" if allowed_keys.contains(&"name") => {
                if parsed.name.is_some() {
                    return Err("duplicate attribute argument: name");
                }
                parsed.name = Some(parse_attribute_string(tokens.get(idx))?);
                idx += 1;
            }
            "id" if allowed_keys.contains(&"id") => {
                if parsed.id.is_some() {
                    return Err("duplicate attribute argument: id");
                }
                parsed.id = Some(parse_attribute_string(tokens.get(idx))?);
                idx += 1;
            }
            "doc" if allowed_keys.contains(&"doc") => {
                if doc_seen {
                    return Err("duplicate attribute argument: doc");
                }
                parsed.doc = parse_attribute_bool(tokens.get(idx))?;
                doc_seen = true;
                idx += 1;
            }
            _ => {
                return Err(unsupported_attribute_message(allowed_keys));
            }
        }

        if idx < tokens.len() {
            match &tokens[idx] {
                TokenTree::Punct(p) if p.as_char() == ',' => idx += 1,
                _ => {
                    return Err(
                        "unsupported attribute arguments, expected comma-separated key/value pairs",
                    );
                }
            }
        }
    }

    Ok(parsed)
}

fn parse_attribute_string(token: Option<&TokenTree>) -> Result<String, &'static str> {
    let raw = match token {
        Some(TokenTree::Literal(value)) => value.to_string(),
        _ => return Err("unsupported attribute arguments, expected key = \"value\""),
    };

    if !raw.starts_with('"') || !raw.ends_with('"') || raw.len() < 2 {
        return Err("attribute values must be string literals");
    }

    Ok(raw[1..raw.len() - 1].to_string())
}

fn parse_attribute_bool(token: Option<&TokenTree>) -> Result<bool, &'static str> {
    match token {
        Some(TokenTree::Ident(value)) if value.to_string() == "true" => Ok(true),
        Some(TokenTree::Ident(value)) if value.to_string() == "false" => Ok(false),
        _ => Err("attribute value for doc must be true or false"),
    }
}

fn unsupported_attribute_message(allowed_keys: &[&str]) -> &'static str {
    match allowed_keys {
        ["name"] => "unsupported attribute argument, expected: name",
        _ => "unsupported attribute argument, expected one of: name, id, doc",
    }
}

fn transform_fn(attrs: AttrArgs, input: TokenStream) -> TokenStream {
    let mut tokens: Vec<TokenTree> = input.into_iter().collect();

    let fn_index = tokens
        .iter()
        .position(|t| matches!(t, TokenTree::Ident(id) if id.to_string() == "fn"));
    let Some(fn_index) = fn_index else {
        return compile_error("#[allure_test] can be applied only to functions");
    };

    let is_async = tokens[..fn_index]
        .iter()
        .any(|t| matches!(t, TokenTree::Ident(id) if id.to_string() == "async"));

    let should_panic = parse_should_panic_config(&tokens[..fn_index]);
    let doc_description = if attrs.doc {
        doc_comments::description(&tokens[..fn_index])
    } else {
        None
    };

    let fn_name = tokens.iter().skip(fn_index + 1).find_map(|t| match t {
        TokenTree::Ident(id) => Some(id.to_string()),
        _ => None,
    });
    let Some(fn_name) = fn_name else {
        return compile_error("failed to parse function name");
    };

    let body_index = tokens.iter().position(
        |t| matches!(t, TokenTree::Group(group) if group.delimiter() == Delimiter::Brace),
    );
    let Some(body_index) = body_index else {
        return compile_error("failed to parse function body");
    };

    let return_type = explicit_return_type(&tokens, fn_index, body_index);
    if should_panic.enabled
        && return_type
            .as_ref()
            .is_some_and(|return_type| !is_unit_return_type(&return_type.ty))
    {
        return compile_error(
            "#[allure_test] supports #[should_panic] only for tests that return ()",
        );
    }

    let original_body = match &tokens[body_index] {
        TokenTree::Group(group) => rewrite_asserts(group.stream()).to_string(),
        _ => return compile_error("failed to parse function body"),
    };
    if should_panic.enabled {
        tokens = remove_should_panic_attrs(tokens);
    }
    let fn_index = tokens
        .iter()
        .position(|t| matches!(t, TokenTree::Ident(id) if id.to_string() == "fn"));
    let Some(fn_index) = fn_index else {
        return compile_error("#[allure_test] can be applied only to functions");
    };
    let body_index = tokens.iter().position(
        |t| matches!(t, TokenTree::Group(group) if group.delimiter() == Delimiter::Brace),
    );
    let Some(mut body_index) = body_index else {
        return compile_error("failed to parse function body");
    };
    let return_type = explicit_return_type(&tokens, fn_index, body_index);

    let test_name = attrs.name.unwrap_or(fn_name.clone());
    let returns_termination_wrapper = return_type.is_some() && !should_panic.enabled;
    let termination_wrapper_name =
        returns_termination_wrapper.then(|| generated_termination_wrapper_name(&fn_name));
    let test_return_type = return_type
        .as_ref()
        .map(|return_type| return_type.ty.as_str())
        .unwrap_or("()");
    let skipped_return = if returns_termination_wrapper {
        let wrapper_name = termination_wrapper_name
            .as_ref()
            .expect("termination wrapper name should be available");
        format!("return {wrapper_name}::skipped();")
    } else {
        match return_type.as_ref() {
            Some(return_type) => format!(
                "return <{} as ::allure_cargotest::__private::AllureTestOutcome>::successful();",
                return_type.ty
            ),
            None => "return;".to_string(),
        }
    };
    let allure_id_option = match attrs.id.as_ref() {
        Some(id) => format!(".with_allure_id({id:?})"),
        None => String::new(),
    };
    let doc_description_option = match doc_description.as_ref() {
        Some(description) => format!(".with_description({description:?})"),
        None => String::new(),
    };

    if returns_termination_wrapper {
        let Some(return_type) = &return_type else {
            return compile_error("failed to parse function return type");
        };
        let wrapper_name = termination_wrapper_name
            .as_ref()
            .expect("termination wrapper name should be available");
        let replacement_src = format!("-> {wrapper_name}");
        let replacement: Vec<TokenTree> = match replacement_src.parse::<TokenStream>() {
            Ok(stream) => stream.into_iter().collect(),
            Err(_) => return compile_error("failed to generate transformed test return type"),
        };
        tokens.splice(return_type.arrow_start..return_type.end, replacement);
        body_index = tokens
            .iter()
            .position(
                |t| matches!(t, TokenTree::Group(group) if group.delimiter() == Delimiter::Brace),
            )
            .unwrap_or(body_index);
    }

    let wrapped_body_src = if should_panic.enabled && is_async {
        format!(
            "{{
  let __allure_full_name = format!(\"{{}}::{{}}\", module_path!(), {fn_name:?});
  if !::allure_cargotest::__private::is_selected(Some(&__allure_full_name), None, None) {{
    {skipped_return}
  }}
  let __allure_options = ::allure_cargotest::__private::TestOptions::new({test_name:?})
    .with_full_name(__allure_full_name.clone())
    .with_source(file!(), env!(\"CARGO_MANIFEST_DIR\"), module_path!())
    .with_panic_status(::allure_cargotest::Status::Failed)
    {doc_description_option}
    {allure_id_option};
  return ::allure_cargotest::__private::test_with_outcome_async(__allure_options, async move {{
    let allure = ::allure_cargotest::__private::current_allure()
      .expect(\"allure context should be available\");
    let _ = &allure;
    let __allure_result = ::allure_cargotest::__private::catch_unwind_async(async move {{ {original_body} }}).await;
    match __allure_result {{
      Ok(()) => panic!(\"expected panic but none occurred\"),
      Err(__allure_payload) => {{
        let __allure_message = if let Some(__allure_msg) = __allure_payload.downcast_ref::<&str>() {{
          (*__allure_msg).to_string()
        }} else if let Some(__allure_msg) = __allure_payload.downcast_ref::<String>() {{
          __allure_msg.clone()
        }} else {{
          \"panic without string payload\".to_string()
        }};
        {}
      }}
    }}
  }}).await;
}}",
            expected_panic_check(&should_panic.expected)
        )
    } else if should_panic.enabled {
        format!(
            "{{
  let __allure_full_name = format!(\"{{}}::{{}}\", module_path!(), {fn_name:?});
  if !::allure_cargotest::__private::is_selected(Some(&__allure_full_name), None, None) {{
    {skipped_return}
  }}
  let __allure_options = ::allure_cargotest::__private::TestOptions::new({test_name:?})
    .with_full_name(__allure_full_name.clone())
    .with_source(file!(), env!(\"CARGO_MANIFEST_DIR\"), module_path!())
    .with_panic_status(::allure_cargotest::Status::Failed)
    {doc_description_option}
    {allure_id_option};
  return ::allure_cargotest::__private::test_with_outcome(__allure_options, || -> () {{
    let allure = ::allure_cargotest::__private::current_allure()
      .expect(\"allure context should be available\");
    let _ = &allure;
    let __allure_result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {{ {original_body} }}));
    match __allure_result {{
      Ok(()) => panic!(\"expected panic but none occurred\"),
      Err(__allure_payload) => {{
        let __allure_message = if let Some(__allure_msg) = __allure_payload.downcast_ref::<&str>() {{
          (*__allure_msg).to_string()
        }} else if let Some(__allure_msg) = __allure_payload.downcast_ref::<String>() {{
          __allure_msg.clone()
        }} else {{
          \"panic without string payload\".to_string()
        }};
        {}
      }}
    }}
  }});
}}",
            expected_panic_check(&should_panic.expected)
        )
    } else if returns_termination_wrapper && is_async {
        let Some(return_type) = &return_type else {
            return compile_error("failed to parse function return type");
        };
        let wrapper_name = termination_wrapper_name
            .as_ref()
            .expect("termination wrapper name should be available");
        let async_output_src = termination_async_output_src(return_type, &original_body);
        format!(
            "{{
  let __allure_full_name = format!(\"{{}}::{{}}\", module_path!(), {fn_name:?});
  if !::allure_cargotest::__private::is_selected(Some(&__allure_full_name), None, None) {{
    {skipped_return}
  }}
  let __allure_options = ::allure_cargotest::__private::TestOptions::new({test_name:?})
    .with_full_name(__allure_full_name.clone())
    .with_source(file!(), env!(\"CARGO_MANIFEST_DIR\"), module_path!())
    .with_panic_status(::allure_cargotest::Status::Failed)
    {doc_description_option}
    {allure_id_option};
  let __allure_runtime_test = ::allure_cargotest::__private::start_runtime_test_with_options(__allure_options);
  let __allure_result = ::allure_cargotest::__private::run_runtime_test_future(
    &__allure_runtime_test,
    async move {{
      {async_output_src}
    }},
  ).await;
  match __allure_result {{
    Ok(__allure_value) => return {wrapper_name}::completed(__allure_runtime_test, __allure_value),
    Err(__allure_payload) => ::allure_cargotest::__private::finish_runtime_test_panic(__allure_runtime_test, __allure_payload),
  }}
}}"
        )
    } else if is_async {
        format!(
            "{{
  let __allure_full_name = format!(\"{{}}::{{}}\", module_path!(), {fn_name:?});
  if !::allure_cargotest::__private::is_selected(Some(&__allure_full_name), None, None) {{
    {skipped_return}
  }}
  let __allure_options = ::allure_cargotest::__private::TestOptions::new({test_name:?})
    .with_full_name(__allure_full_name.clone())
    .with_source(file!(), env!(\"CARGO_MANIFEST_DIR\"), module_path!())
    .with_panic_status(::allure_cargotest::Status::Failed)
    {doc_description_option}
    {allure_id_option};
  return ::allure_cargotest::__private::test_with_outcome_async(__allure_options, async move {{
    let __allure_output: {test_return_type} = {{
      let allure = ::allure_cargotest::__private::current_allure()
        .expect(\"allure context should be available\");
      let _ = &allure;
      {original_body}
    }};
    __allure_output
  }}).await;
}}"
        )
    } else if returns_termination_wrapper {
        let Some(return_type) = &return_type else {
            return compile_error("failed to parse function return type");
        };
        let wrapper_name = termination_wrapper_name
            .as_ref()
            .expect("termination wrapper name should be available");
        let sync_closure_src = termination_sync_closure_src(return_type, &original_body);
        format!(
            "{{
  let __allure_full_name = format!(\"{{}}::{{}}\", module_path!(), {fn_name:?});
  if !::allure_cargotest::__private::is_selected(Some(&__allure_full_name), None, None) {{
    {skipped_return}
  }}
  let __allure_options = ::allure_cargotest::__private::TestOptions::new({test_name:?})
    .with_full_name(__allure_full_name.clone())
    .with_source(file!(), env!(\"CARGO_MANIFEST_DIR\"), module_path!())
    .with_panic_status(::allure_cargotest::Status::Failed)
    {doc_description_option}
    {allure_id_option};
  let __allure_runtime_test = ::allure_cargotest::__private::start_runtime_test_with_options(__allure_options);
  let __allure_result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe({sync_closure_src}));
  match __allure_result {{
    Ok(__allure_value) => return {wrapper_name}::completed(__allure_runtime_test, __allure_value),
    Err(__allure_payload) => ::allure_cargotest::__private::finish_runtime_test_panic(__allure_runtime_test, __allure_payload),
  }}
}}"
        )
    } else {
        format!(
            "{{
  let __allure_full_name = format!(\"{{}}::{{}}\", module_path!(), {fn_name:?});
  if !::allure_cargotest::__private::is_selected(Some(&__allure_full_name), None, None) {{
    {skipped_return}
  }}
  let __allure_options = ::allure_cargotest::__private::TestOptions::new({test_name:?})
    .with_full_name(__allure_full_name.clone())
    .with_source(file!(), env!(\"CARGO_MANIFEST_DIR\"), module_path!())
    .with_panic_status(::allure_cargotest::Status::Failed)
    {doc_description_option}
    {allure_id_option};
  return ::allure_cargotest::__private::test_with_outcome(__allure_options, || -> {test_return_type} {{
    let allure = ::allure_cargotest::__private::current_allure()
      .expect(\"allure context should be available\");
    let _ = &allure;
    {original_body}
  }});
}}"
        )
    };

    let wrapped_body_stream: TokenStream = match wrapped_body_src.parse() {
        Ok(stream) => stream,
        Err(_) => return compile_error("failed to generate transformed test body"),
    };
    let wrapped_group = match wrapped_body_stream.into_iter().next() {
        Some(TokenTree::Group(group)) => group,
        _ => return compile_error("failed to generate transformed test body"),
    };

    tokens[body_index] = TokenTree::Group(Group::new(Delimiter::Brace, wrapped_group.stream()));

    let mut output = TokenStream::new();
    if returns_termination_wrapper {
        let Some(return_type) = &return_type else {
            return compile_error("failed to parse function return type");
        };
        let wrapper_name = termination_wrapper_name
            .as_ref()
            .expect("termination wrapper name should be available");
        let wrapper_stream: TokenStream = match termination_wrapper_src(return_type, wrapper_name)
            .parse()
        {
            Ok(stream) => stream,
            Err(_) => return compile_error("failed to generate transformed test return wrapper"),
        };
        output.extend(wrapper_stream);
    }
    output.extend(TokenStream::from_iter(tokens));
    output
}

fn parse_should_panic_config(tokens: &[TokenTree]) -> ShouldPanicConfig {
    let mut index = 0;
    while index + 1 < tokens.len() {
        let Some(TokenTree::Punct(pound)) = tokens.get(index) else {
            index += 1;
            continue;
        };
        if pound.as_char() != '#' {
            index += 1;
            continue;
        }

        let Some(TokenTree::Group(group)) = tokens.get(index + 1) else {
            index += 1;
            continue;
        };
        if group.delimiter() != Delimiter::Bracket {
            index += 2;
            continue;
        }

        let mut attr_tokens = group.stream().into_iter();
        let Some(TokenTree::Ident(name)) = attr_tokens.next() else {
            index += 2;
            continue;
        };
        if name.to_string() != "should_panic" {
            index += 2;
            continue;
        }

        let expected = attr_tokens.find_map(|token| match token {
            TokenTree::Group(arguments) if arguments.delimiter() == Delimiter::Parenthesis => {
                parse_should_panic_expected(arguments.stream())
            }
            _ => None,
        });
        return ShouldPanicConfig {
            enabled: true,
            expected,
        };
    }

    ShouldPanicConfig {
        enabled: false,
        expected: None,
    }
}

fn remove_should_panic_attrs(tokens: Vec<TokenTree>) -> Vec<TokenTree> {
    let mut output = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if is_should_panic_attr_at(&tokens, index) {
            index += 2;
            continue;
        }

        output.push(tokens[index].clone());
        index += 1;
    }

    output
}

fn is_should_panic_attr_at(tokens: &[TokenTree], index: usize) -> bool {
    let Some(TokenTree::Punct(pound)) = tokens.get(index) else {
        return false;
    };
    if pound.as_char() != '#' {
        return false;
    }

    let Some(TokenTree::Group(group)) = tokens.get(index + 1) else {
        return false;
    };
    if group.delimiter() != Delimiter::Bracket {
        return false;
    }

    matches!(
        group.stream().into_iter().next(),
        Some(TokenTree::Ident(name)) if name.to_string() == "should_panic"
    )
}

fn parse_should_panic_expected(tokens: TokenStream) -> Option<String> {
    let parsed: Vec<TokenTree> = tokens.into_iter().collect();
    for window in parsed.windows(3) {
        match window {
            [TokenTree::Ident(name), TokenTree::Punct(eq), TokenTree::Literal(value)]
                if name.to_string() == "expected" && eq.as_char() == '=' =>
            {
                let raw = value.to_string();
                if raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2 {
                    return Some(raw[1..raw.len() - 1].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn expected_panic_check(expected: &Option<String>) -> String {
    match expected {
        Some(expected) => format!(
            "if !__allure_message.contains({expected:?}) {{
          panic!(\"panic message mismatch: expected substring {{:?}}, got {{:?}}\", {expected:?}, __allure_message);
        }}"
        ),
        None => "let _ = __allure_message;".to_string(),
    }
}

fn generated_termination_wrapper_name(fn_name: &str) -> String {
    let mut sanitized = fn_name.strip_prefix("r#").unwrap_or(fn_name).to_string();
    sanitized.retain(|ch| ch == '_' || ch.is_ascii_alphanumeric());
    format!("__AllureTerminationResult_{sanitized}")
}

fn termination_wrapper_src(return_type: &ExplicitReturnType, wrapper_name: &str) -> String {
    match return_type.kind {
        ReturnTypeKind::Concrete => concrete_termination_wrapper_src(&return_type.ty, wrapper_name),
        ReturnTypeKind::Opaque => boxed_termination_wrapper_src(wrapper_name),
        ReturnTypeKind::Never => never_termination_wrapper_src(wrapper_name),
    }
}

fn concrete_termination_wrapper_src(return_type: &str, wrapper_name: &str) -> String {
    format!(
        r#"
#[allow(non_camel_case_types)]
struct {wrapper_name} {{
  __allure_runtime_test: Option<::allure_cargotest::__private::RuntimeTest>,
  __allure_value: Option<{return_type}>,
}}

impl {wrapper_name} {{
  fn skipped() -> Self {{
    Self {{
      __allure_runtime_test: None,
      __allure_value: None,
    }}
  }}

  fn completed(
    __allure_runtime_test: ::allure_cargotest::__private::RuntimeTest,
    __allure_value: {return_type},
  ) -> Self {{
    Self {{
      __allure_runtime_test: Some(__allure_runtime_test),
      __allure_value: Some(__allure_value),
    }}
  }}
}}

impl ::std::process::Termination for {wrapper_name} {{
  fn report(mut self) -> ::std::process::ExitCode {{
    let Some(__allure_runtime_test) = self.__allure_runtime_test.take() else {{
      return ::std::process::ExitCode::SUCCESS;
    }};
    let Some(__allure_value) = self.__allure_value.take() else {{
      return ::std::process::ExitCode::SUCCESS;
    }};
    use ::allure_cargotest::__private::FinishAllureOutcome as _;
    let mut __allure_probe = ::allure_cargotest::__private::AllureOutcomeProbe::new(__allure_value);
    let __allure_report = match __allure_probe.finish_allure_outcome() {{
      Ok(__allure_report) => __allure_report,
      Err(__allure_payload) => {{
        ::allure_cargotest::__private::finish_runtime_test_panic(
          __allure_runtime_test,
          __allure_payload,
        );
      }}
    }};
    ::allure_cargotest::__private::finish_runtime_test_status(
      __allure_runtime_test,
      __allure_report.status,
      __allure_report.details,
    );
    __allure_report.exit_code
  }}
}}
"#
    )
}

fn boxed_termination_wrapper_src(wrapper_name: &str) -> String {
    let boxed_trait_name = format!("{wrapper_name}_BoxedTermination");
    r#"
#[allow(non_camel_case_types)]
struct __ALLURE_WRAPPER_NAME__ {
  __allure_runtime_test: Option<::allure_cargotest::__private::RuntimeTest>,
  __allure_value: Option<Box<dyn __ALLURE_BOXED_TRAIT_NAME__>>,
}

#[allow(non_camel_case_types)]
trait __ALLURE_BOXED_TRAIT_NAME__ {
  fn __allure_finish(
    self: Box<Self>,
  ) -> ::std::thread::Result<::allure_cargotest::__private::AllureOutcomeReport>;
}

impl<R> __ALLURE_BOXED_TRAIT_NAME__ for R
where
  R: ::std::process::Termination + 'static,
{
  fn __allure_finish(
    self: Box<Self>,
  ) -> ::std::thread::Result<::allure_cargotest::__private::AllureOutcomeReport> {
    let __allure_exit_code =
      ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| (*self).report()))?;
    let (__allure_status, __allure_details) =
      if __allure_exit_code == ::std::process::ExitCode::SUCCESS {
        (::allure_cargotest::Status::Passed, None)
      } else {
        (
          ::allure_cargotest::Status::Broken,
          Some(::allure_cargotest::__private::status_details_for_message(
            "test returned unsuccessful Termination status".to_string(),
          )),
        )
      };
    Ok(::allure_cargotest::__private::AllureOutcomeReport {
      exit_code: __allure_exit_code,
      status: __allure_status,
      details: __allure_details,
    })
  }
}

impl __ALLURE_WRAPPER_NAME__ {
  fn skipped() -> Self {
    Self {
      __allure_runtime_test: None,
      __allure_value: None,
    }
  }

  fn completed<R>(
    __allure_runtime_test: ::allure_cargotest::__private::RuntimeTest,
    __allure_value: R,
  ) -> Self
  where
    R: ::std::process::Termination + 'static,
  {
    Self {
      __allure_runtime_test: Some(__allure_runtime_test),
      __allure_value: Some(Box::new(__allure_value)),
    }
  }
}

impl ::std::process::Termination for __ALLURE_WRAPPER_NAME__ {
  fn report(mut self) -> ::std::process::ExitCode {
    let Some(__allure_runtime_test) = self.__allure_runtime_test.take() else {
      return ::std::process::ExitCode::SUCCESS;
    };
    let Some(__allure_value) = self.__allure_value.take() else {
      return ::std::process::ExitCode::SUCCESS;
    };
    let __allure_report = match __allure_value.__allure_finish() {
      Ok(__allure_report) => __allure_report,
      Err(__allure_payload) => {
        ::allure_cargotest::__private::finish_runtime_test_panic(
          __allure_runtime_test,
          __allure_payload,
        );
      }
    };
    ::allure_cargotest::__private::finish_runtime_test_status(
      __allure_runtime_test,
      __allure_report.status,
      __allure_report.details,
    );
    __allure_report.exit_code
  }
}
"#
    .replace("__ALLURE_WRAPPER_NAME__", wrapper_name)
    .replace("__ALLURE_BOXED_TRAIT_NAME__", &boxed_trait_name)
}

fn never_termination_wrapper_src(wrapper_name: &str) -> String {
    r#"
#[allow(non_camel_case_types)]
struct __ALLURE_WRAPPER_NAME__ {
  __allure_runtime_test: Option<::allure_cargotest::__private::RuntimeTest>,
}

impl __ALLURE_WRAPPER_NAME__ {
  fn skipped() -> Self {
    Self {
      __allure_runtime_test: None,
    }
  }

  fn completed(
    __allure_runtime_test: ::allure_cargotest::__private::RuntimeTest,
    _completed: (),
  ) -> Self {
    Self {
      __allure_runtime_test: Some(__allure_runtime_test),
    }
  }
}

impl ::std::process::Termination for __ALLURE_WRAPPER_NAME__ {
  fn report(mut self) -> ::std::process::ExitCode {
    let Some(__allure_runtime_test) = self.__allure_runtime_test.take() else {
      return ::std::process::ExitCode::SUCCESS;
    };
    ::allure_cargotest::__private::finish_runtime_test_status(
      __allure_runtime_test,
      ::allure_cargotest::Status::Broken,
      Some(::allure_cargotest::__private::status_details_for_message(
        "never-returning test completed".to_string(),
      )),
    );
    ::std::process::ExitCode::FAILURE
  }
}
"#
    .replace("__ALLURE_WRAPPER_NAME__", wrapper_name)
}

fn termination_sync_closure_src(return_type: &ExplicitReturnType, original_body: &str) -> String {
    let body = format!(
        "let _current_allure = ::allure_cargotest::__private::push_runtime_test_allure(&__allure_runtime_test);
    let allure = ::allure_cargotest::__private::current_allure()
      .expect(\"allure context should be available\");
    let _ = &allure;
    {original_body}"
    );

    match return_type.kind {
        ReturnTypeKind::Concrete => format!("|| -> {} {{ {body} }}", return_type.ty),
        ReturnTypeKind::Opaque => format!("|| {{ {body} }}"),
        ReturnTypeKind::Never => format!("|| {{ {body}; }}"),
    }
}

fn termination_async_output_src(return_type: &ExplicitReturnType, original_body: &str) -> String {
    let body = format!(
        "let allure = ::allure_cargotest::__private::current_allure()
        .expect(\"allure context should be available\");
      let _ = &allure;
      {original_body}"
    );

    match return_type.kind {
        ReturnTypeKind::Concrete => format!(
            "let __allure_output: {} = {{ {body} }};
      __allure_output",
            return_type.ty
        ),
        ReturnTypeKind::Opaque => format!(
            "let __allure_output = {{ {body} }};
      __allure_output"
        ),
        ReturnTypeKind::Never => format!("{{ {body}; }}"),
    }
}

fn explicit_return_type(
    tokens: &[TokenTree],
    fn_index: usize,
    body_index: usize,
) -> Option<ExplicitReturnType> {
    for index in fn_index..body_index.saturating_sub(1) {
        if let [TokenTree::Punct(first), TokenTree::Punct(second)] = &tokens[index..index + 2] {
            if first.as_char() == '-' && second.as_char() == '>' {
                let start = index + 2;
                let end = tokens[start..]
                    .iter()
                    .position(|token| {
                        matches!(token, TokenTree::Ident(ident) if ident.to_string() == "where")
                    })
                    .map_or(body_index, |offset| start + offset);

                let return_tokens = &tokens[start..end];
                let ty = TokenStream::from_iter(return_tokens.iter().cloned()).to_string();
                let kind = return_type_kind(return_tokens, &ty);

                return Some(ExplicitReturnType {
                    ty,
                    arrow_start: index,
                    end,
                    kind,
                });
            }
        }
    }
    None
}

fn return_type_kind(tokens: &[TokenTree], return_type: &str) -> ReturnTypeKind {
    if is_never_return_type(return_type) {
        ReturnTypeKind::Never
    } else if matches!(
        tokens.first(),
        Some(TokenTree::Ident(ident)) if ident.to_string() == "impl"
    ) {
        ReturnTypeKind::Opaque
    } else {
        ReturnTypeKind::Concrete
    }
}

fn is_unit_return_type(return_type: &str) -> bool {
    return_type
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        == "()"
}

fn is_never_return_type(return_type: &str) -> bool {
    return_type
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        == "!"
}

fn compile_error(message: &str) -> TokenStream {
    format!("compile_error!({message:?});")
        .parse()
        .unwrap_or_default()
}

fn transform_step_fn(attrs: StepAttrArgs, input: TokenStream) -> TokenStream {
    let mut tokens: Vec<TokenTree> = input.into_iter().collect();

    let fn_index = tokens
        .iter()
        .position(|t| matches!(t, TokenTree::Ident(id) if id.to_string() == "fn"));
    let Some(fn_index) = fn_index else {
        return compile_error("#[step] can be applied only to functions");
    };

    let fn_name = tokens.iter().skip(fn_index + 1).find_map(|t| match t {
        TokenTree::Ident(id) => Some(id.to_string()),
        _ => None,
    });
    let Some(fn_name) = fn_name else {
        return compile_error("failed to parse function name");
    };

    let body_index = tokens.iter().position(
        |t| matches!(t, TokenTree::Group(group) if group.delimiter() == Delimiter::Brace),
    );
    let Some(body_index) = body_index else {
        return compile_error("failed to parse function body");
    };

    let original_body = match &tokens[body_index] {
        TokenTree::Group(group) => rewrite_asserts(group.stream()).to_string(),
        _ => return compile_error("failed to parse function body"),
    };

    let step_name = attrs.name.unwrap_or(fn_name);
    let wrapped_body_src = format!(
        "{{
  let __allure_step_name = {step_name:?};
  match ::allure_cargotest::__private::current_allure() {{
    Some(__allure_step_allure) => {{
      let __allure_step_scope = ::allure_cargotest::__private::begin_step_scope(
        __allure_step_allure,
        __allure_step_name,
      );
      {original_body}
    }}
    None => {{
      {original_body}
    }}
  }}
}}"
    );

    let wrapped_body_stream: TokenStream = match wrapped_body_src.parse() {
        Ok(stream) => stream,
        Err(_) => return compile_error("failed to generate transformed function body"),
    };
    let wrapped_group = match wrapped_body_stream.into_iter().next() {
        Some(TokenTree::Group(group)) => group,
        _ => return compile_error("failed to generate transformed function body"),
    };

    tokens[body_index] = TokenTree::Group(Group::new(Delimiter::Brace, wrapped_group.stream()));

    TokenStream::from_iter(tokens)
}

fn transform_log_asserts_fn(input: TokenStream) -> TokenStream {
    let mut tokens: Vec<TokenTree> = input.into_iter().collect();

    let fn_index = tokens
        .iter()
        .position(|t| matches!(t, TokenTree::Ident(id) if id.to_string() == "fn"));
    if fn_index.is_none() {
        return compile_error("#[log_asserts] can be applied only to functions");
    }

    let body_index = tokens.iter().position(
        |t| matches!(t, TokenTree::Group(group) if group.delimiter() == Delimiter::Brace),
    );
    let Some(body_index) = body_index else {
        return compile_error("failed to parse function body");
    };

    let rewritten_body = match &tokens[body_index] {
        TokenTree::Group(group) => {
            let mut rewritten = Group::new(Delimiter::Brace, rewrite_asserts(group.stream()));
            rewritten.set_span(group.span());
            TokenTree::Group(rewritten)
        }
        _ => return compile_error("failed to parse function body"),
    };

    tokens[body_index] = rewritten_body;
    TokenStream::from_iter(tokens)
}

#[derive(Clone, Copy)]
enum AssertMacroKind {
    Assert,
    AssertEq,
    AssertNe,
    DebugAssert,
    DebugAssertEq,
    DebugAssertNe,
}

impl AssertMacroKind {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "assert" => Some(Self::Assert),
            "assert_eq" => Some(Self::AssertEq),
            "assert_ne" => Some(Self::AssertNe),
            "debug_assert" => Some(Self::DebugAssert),
            "debug_assert_eq" => Some(Self::DebugAssertEq),
            "debug_assert_ne" => Some(Self::DebugAssertNe),
            _ => None,
        }
    }

    fn macro_name(self) -> &'static str {
        match self {
            Self::Assert => "assert",
            Self::AssertEq => "assert_eq",
            Self::AssertNe => "assert_ne",
            Self::DebugAssert => "debug_assert",
            Self::DebugAssertEq => "debug_assert_eq",
            Self::DebugAssertNe => "debug_assert_ne",
        }
    }

    fn is_debug(self) -> bool {
        matches!(
            self,
            Self::DebugAssert | Self::DebugAssertEq | Self::DebugAssertNe
        )
    }
}

fn rewrite_asserts(stream: TokenStream) -> TokenStream {
    let tokens: Vec<TokenTree> = stream.into_iter().collect();
    let mut output = Vec::new();
    let mut index = 0;

    while index < tokens.len() {
        if let Some((name, group)) = macro_invocation_at(&tokens, index) {
            if let Some(kind) = AssertMacroKind::from_name(&name) {
                output.extend(rewrite_assert_macro(kind, group.stream()));
            } else {
                output.push(tokens[index].clone());
                output.push(tokens[index + 1].clone());
                output.push(tokens[index + 2].clone());
            }
            index += 3;
            continue;
        }

        match &tokens[index] {
            TokenTree::Group(group) => {
                let mut rewritten = Group::new(group.delimiter(), rewrite_asserts(group.stream()));
                rewritten.set_span(group.span());
                output.push(TokenTree::Group(rewritten));
            }
            token => output.push(token.clone()),
        }
        index += 1;
    }

    TokenStream::from_iter(output)
}

fn macro_invocation_at(tokens: &[TokenTree], index: usize) -> Option<(String, Group)> {
    let Some(TokenTree::Ident(name)) = tokens.get(index) else {
        return None;
    };
    let Some(TokenTree::Punct(bang)) = tokens.get(index + 1) else {
        return None;
    };
    if bang.as_char() != '!' {
        return None;
    }
    let Some(TokenTree::Group(group)) = tokens.get(index + 2) else {
        return None;
    };
    if group.delimiter() != Delimiter::Parenthesis {
        return None;
    }

    Some((name.to_string(), group.clone()))
}

fn rewrite_assert_macro(kind: AssertMacroKind, args: TokenStream) -> TokenStream {
    let args_string = args.to_string();
    let generated = match kind {
        AssertMacroKind::Assert | AssertMacroKind::DebugAssert => {
            generate_assert_code(kind, args.clone())
        }
        AssertMacroKind::AssertEq
        | AssertMacroKind::AssertNe
        | AssertMacroKind::DebugAssertEq
        | AssertMacroKind::DebugAssertNe => generate_assert_cmp_code(kind, args.clone()),
    };

    generated
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| original_assert_invocation(kind.macro_name(), &args_string))
}

fn original_assert_invocation(name: &str, args: &str) -> TokenStream {
    format!("{name}!({args})").parse().unwrap_or_default()
}

fn split_macro_args(args: TokenStream) -> Vec<TokenStream> {
    let mut parts = Vec::new();
    let mut current = Vec::new();

    for token in args {
        match &token {
            TokenTree::Punct(punct) if punct.as_char() == ',' => {
                parts.push(TokenStream::from_iter(current));
                current = Vec::new();
            }
            _ => current.push(token),
        }
    }

    parts.push(TokenStream::from_iter(current));
    parts
}

fn is_empty_stream(stream: &TokenStream) -> bool {
    stream.clone().into_iter().next().is_none()
}

fn join_token_streams(streams: &[TokenStream]) -> String {
    streams
        .iter()
        .map(TokenStream::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn generate_assert_code(kind: AssertMacroKind, args: TokenStream) -> Option<String> {
    let parts = split_macro_args(args.clone());
    let condition = parts.first()?.clone();
    if is_empty_stream(&condition) {
        return None;
    }

    let condition = condition.to_string();
    let custom_message = if parts.len() > 1 {
        let custom = join_token_streams(&parts[1..]);
        if custom.trim().is_empty() {
            None
        } else {
            Some(format!("format!(\"{{}}\", format_args!({custom}))"))
        }
    } else {
        None
    };
    let message = custom_message
        .unwrap_or_else(|| format!("format!(\"assertion failed: {{}}\", stringify!({condition}))"));
    let name = format!(
        "concat!(\"{}!(\", stringify!({condition}), \")\")",
        kind.macro_name()
    );

    let mut instrumented = String::new();
    instrumented.push_str("if ");
    instrumented.push_str(&condition);
    instrumented.push_str(" { ::allure_cargotest::__private::record_assertion_pass(");
    instrumented.push_str(&name);
    instrumented.push_str("); } else { let __allure_assert_message = ");
    instrumented.push_str(&message);
    instrumented.push_str("; ::allure_cargotest::__private::fail_assertion(");
    instrumented.push_str(&name);
    instrumented.push_str(
        ", __allure_assert_message, Some(\"false\".to_string()), Some(\"true\".to_string())); }",
    );

    Some(runtime_assert_code(
        instrumented,
        original_assert_source(kind.macro_name(), &args.to_string()),
        kind.is_debug(),
    ))
}

fn generate_assert_cmp_code(kind: AssertMacroKind, args: TokenStream) -> Option<String> {
    let parts = split_macro_args(args.clone());
    if parts.len() < 2 || is_empty_stream(&parts[0]) || is_empty_stream(&parts[1]) {
        return None;
    }

    let left = parts[0].to_string();
    let right = parts[1].to_string();
    let is_ne = matches!(
        kind,
        AssertMacroKind::AssertNe | AssertMacroKind::DebugAssertNe
    );
    let operator = if is_ne { "!=" } else { "==" };
    let default_message = if is_ne {
        "format!(\"assertion `left != right` failed\\n  left: `{:?}`,\\n right: `{:?}`\", __allure_assert_left, __allure_assert_right)"
    } else {
        "format!(\"assertion `left == right` failed\\n  left: `{:?}`,\\n right: `{:?}`\", __allure_assert_left, __allure_assert_right)"
    };
    let message = if parts.len() > 2 {
        let custom = join_token_streams(&parts[2..]);
        if custom.trim().is_empty() {
            default_message.to_string()
        } else {
            format!("format!(\"{{}}\", format_args!({custom}))")
        }
    } else {
        default_message.to_string()
    };
    let name = format!(
        "concat!(\"{}!(\", stringify!({left}), \", \", stringify!({right}), \")\")",
        kind.macro_name()
    );

    let mut instrumented = String::new();
    instrumented.push_str("match (&(");
    instrumented.push_str(&left);
    instrumented.push_str("), &(");
    instrumented.push_str(&right);
    instrumented.push_str(
        ")) { (__allure_assert_left, __allure_assert_right) => { if *__allure_assert_left ",
    );
    instrumented.push_str(operator);
    instrumented.push_str(
        " *__allure_assert_right { ::allure_cargotest::__private::record_assertion_pass(",
    );
    instrumented.push_str(&name);
    instrumented.push_str("); } else { let __allure_assert_message = ");
    instrumented.push_str(&message);
    instrumented.push_str("; ::allure_cargotest::__private::fail_assertion(");
    instrumented.push_str(&name);
    instrumented.push_str(", __allure_assert_message, Some(format!(\"{:?}\", __allure_assert_left)), Some(format!(\"{:?}\", __allure_assert_right))); } } }");

    Some(runtime_assert_code(
        instrumented,
        original_assert_source(kind.macro_name(), &args.to_string()),
        kind.is_debug(),
    ))
}

fn original_assert_source(name: &str, args: &str) -> String {
    format!("{name}!({args});")
}

fn runtime_assert_code(instrumented: String, original: String, debug_only: bool) -> String {
    let code = format!(
        "{{ if ::allure_cargotest::__private::log_asserts_enabled(env!(\"CARGO_MANIFEST_DIR\")) {{ {instrumented} }} else {{ {original} }} }}"
    );
    if debug_only {
        format!("{{ if cfg!(debug_assertions) {{ {code} }} }}")
    } else {
        code
    }
}
