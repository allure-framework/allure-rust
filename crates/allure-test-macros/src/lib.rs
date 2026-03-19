use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

struct ShouldPanicConfig {
    enabled: bool,
    expected: Option<String>,
}

#[derive(Debug)]
struct AttrArgs {
    name: Option<String>,
    id: Option<String>,
}

#[derive(Debug)]
struct StepAttrArgs {
    name: Option<String>,
}

#[proc_macro_attribute]
pub fn step(args: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = match parse_step_args(args) {
        Ok(attrs) => attrs,
        Err(err) => return compile_error(err),
    };

    transform_step_fn(attrs, input)
}

#[proc_macro_attribute]
pub fn allure_test(args: TokenStream, input: TokenStream) -> TokenStream {
    let attrs = match parse_args(args) {
        Ok(attrs) => attrs,
        Err(err) => return compile_error(err),
    };

    transform_fn(attrs, input)
}

fn parse_step_args(args: TokenStream) -> Result<StepAttrArgs, &'static str> {
    let attrs = parse_kv_args(args, &["name"])?;
    Ok(StepAttrArgs { name: attrs.name })
}

fn parse_args(args: TokenStream) -> Result<AttrArgs, &'static str> {
    parse_kv_args(args, &["name", "id"])
}

fn parse_kv_args(args: TokenStream, allowed_keys: &[&str]) -> Result<AttrArgs, &'static str> {
    let tokens: Vec<TokenTree> = args.into_iter().collect();
    if tokens.is_empty() {
        return Ok(AttrArgs {
            name: None,
            id: None,
        });
    }

    let mut idx = 0;
    let mut parsed = AttrArgs {
        name: None,
        id: None,
    };

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

        let raw = match tokens.get(idx) {
            Some(TokenTree::Literal(value)) => value.to_string(),
            _ => return Err("unsupported attribute arguments, expected key = \"value\""),
        };
        idx += 1;

        if !raw.starts_with('"') || !raw.ends_with('"') || raw.len() < 2 {
            return Err("attribute values must be string literals");
        }

        let value = raw[1..raw.len() - 1].to_string();
        match key.as_str() {
            "name" if allowed_keys.contains(&"name") => {
                if parsed.name.is_some() {
                    return Err("duplicate attribute argument: name");
                }
                parsed.name = Some(value);
            }
            "id" if allowed_keys.contains(&"id") => {
                if parsed.id.is_some() {
                    return Err("duplicate attribute argument: id");
                }
                parsed.id = Some(value);
            }
            _ => {
                return Err(match allowed_keys {
                    ["name"] => "unsupported attribute argument, expected: name",
                    _ => "unsupported attribute argument, expected one of: name, id",
                });
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

fn transform_fn(attrs: AttrArgs, input: TokenStream) -> TokenStream {
    let mut tokens: Vec<TokenTree> = input.into_iter().collect();

    let fn_index = tokens
        .iter()
        .position(|t| matches!(t, TokenTree::Ident(id) if id.to_string() == "fn"));
    let Some(fn_index) = fn_index else {
        return compile_error("#[allure_test] can be applied only to functions");
    };

    if tokens[..fn_index]
        .iter()
        .any(|t| matches!(t, TokenTree::Ident(id) if id.to_string() == "async"))
    {
        return compile_error("#[allure_test] does not support async functions");
    }

    let should_panic = parse_should_panic_config(&tokens[..fn_index]);

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

    if has_return_type(&tokens[fn_index..body_index]) {
        return compile_error(
            "#[allure_test] currently supports only test functions that return ()",
        );
    }

    let original_body = match &tokens[body_index] {
        TokenTree::Group(group) => group.stream().to_string(),
        _ => return compile_error("failed to parse function body"),
    };

    let test_name = attrs.name.unwrap_or(fn_name.clone());
    let allure_id_setup = match attrs.id.as_ref() {
        Some(id) => format!("allure.id({id:?});"),
        None => String::new(),
    };

    let wrapped_body_src = if should_panic.enabled {
        format!(
            "{{
  let __allure_results_dir = ::std::env::var(\"ALLURE_RESULTS_DIR\")
    .unwrap_or_else(|_| \"target/allure-results\".to_string());
  let __allure_reporter = ::allure_cargotest::CargoTestReporter::new(__allure_results_dir)
    .expect(\"allure reporter should be created\");
  if !__allure_reporter.is_selected({test_name:?}, Some({test_name:?}), None, None) {{
    return;
  }}
  __allure_reporter.run_test_with_result({test_name:?}, |allure| {{
    {allure_id_setup}
    let __allure_result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {{ {original_body} }}));
    match __allure_result {{
      Ok(()) => (
        ::allure_cargotest::Status::Failed,
        Some(::allure_cargotest::StatusDetails {{
          message: Some(\"expected panic but none occurred\".to_string()),
          trace: None,
          actual: None,
          expected: None,
        }}),
        None,
      ),
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
            expected_match_arm(&should_panic.expected)
        )
    } else {
        format!(
            "{{
  let __allure_results_dir = ::std::env::var(\"ALLURE_RESULTS_DIR\")
    .unwrap_or_else(|_| \"target/allure-results\".to_string());
  let __allure_reporter = ::allure_cargotest::CargoTestReporter::new(__allure_results_dir)
    .expect(\"allure reporter should be created\");
  let __allure_full_name = format!(\"{{}}::{{}}\", module_path!(), {fn_name:?});
  __allure_reporter.run_test_with_metadata({test_name:?}, Some(&__allure_full_name), None, None, |allure| {{ {allure_id_setup} {original_body} }});
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

    TokenStream::from_iter(tokens)
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

fn expected_match_arm(expected: &Option<String>) -> String {
    match expected {
        Some(expected) => format!(
            "if __allure_message.contains({expected:?}) {{
          (
            ::allure_cargotest::Status::Passed,
            None,
            Some(__allure_payload),
          )
        }} else {{
          (
            ::allure_cargotest::Status::Failed,
            Some(::allure_cargotest::StatusDetails {{
              message: Some(format!(\"panic message mismatch: expected substring {{:?}}, got {{:?}}\", {expected:?}, __allure_message)),
              trace: None,
              actual: None,
              expected: None,
            }}),
            Some(__allure_payload),
          )
        }}"
        ),
        None => "(
          ::allure_cargotest::Status::Passed,
          None,
          Some(__allure_payload),
        )"
        .to_string(),
    }
}

fn has_return_type(tokens: &[TokenTree]) -> bool {
    for window in tokens.windows(2) {
        if let [TokenTree::Punct(first), TokenTree::Punct(second)] = window {
            if first.as_char() == '-' && second.as_char() == '>' {
                return true;
            }
        }
    }
    false
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
        TokenTree::Group(group) => group.stream().to_string(),
        _ => return compile_error("failed to parse function body"),
    };

    let step_name = attrs.name.unwrap_or(fn_name);
    let wrapped_body_src = format!(
        "{{
  let __allure_step_name = {step_name:?};
  match ::allure_cargotest::__private::current_allure() {{
    Some(__allure_step_allure) => {{
      let __allure_step_guard = __allure_step_allure.step(__allure_step_name);
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
