use super::md5_hex;

fn assert_md5(input: &str, expected: &str) {
    assert_eq!(
        md5_hex(input),
        expected,
        "unexpected MD5 for input: {input:?}"
    );
}

#[test]
fn md5_rfc_1321_test_vectors() {
    assert_md5("", "d41d8cd98f00b204e9800998ecf8427e");
    assert_md5("a", "0cc175b9c0f1b6a831c399e269772661");
    assert_md5("abc", "900150983cd24fb0d6963f7d28e17f72");
    assert_md5("message digest", "f96b697d7cb7938d525a2f31aaf161d0");
    assert_md5(
        "abcdefghijklmnopqrstuvwxyz",
        "c3fcd3d76192e4007dfb496cca67e13b",
    );
    assert_md5(
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
        "d174ab98d277d9f5a5611c2c9f419d9f",
    );
    assert_md5(
        "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
        "57edf4a22be3c955ac49da2e2107b67a",
    );
}

#[test]
fn md5_common_known_strings() {
    assert_md5(
        "The quick brown fox jumps over the lazy dog",
        "9e107d9d372bb6826bd81d3542a419d6",
    );
    assert_md5(
        "The quick brown fox jumps over the lazy dog.",
        "e4d909c290d0fb1ca068ffaddf22cbd0",
    );
    assert_md5("allure", "5d0e3e8958aaf2caae88d62f2cf82f81");
    assert_md5("allure-rust", "3bfa14a24fc809cda9b9be286c7f8fa6");
    assert_md5("testCaseId", "9dc9d789ecbbebd781b2ccc0c6320319");
}

#[test]
fn md5_boundary_lengths_around_single_block_padding() {
    assert_md5(&"a".repeat(55), "ef1772b6dff9a122358552954ad0df65");
    assert_md5(&"a".repeat(56), "3b0c8ac703f828b04c6c197006d17218");
    assert_md5(&"a".repeat(57), "652b906d60af96844ebd21b674f35e93");
    assert_md5(&"a".repeat(63), "b06521f39153d618550606be297466d5");
    assert_md5(&"a".repeat(64), "014842d480b571495a4a0363793f7367");
    assert_md5(&"a".repeat(65), "c743a45e0d2e6a95cb859adae0248435");
}

#[test]
fn md5_binary_and_unicode_inputs() {
    assert_md5("\0", "93b885adfe0da089cdf634904fd59f71");
    assert_md5("\0\u{1}\u{2}\u{3}", "37b59afd592725f9305e484a5d7f5168");
    assert_md5("🧪", "973c5911c2cb66d132f9e87c3496370c");
    assert_md5("こんにちは", "c0e89a293bd36c7a768e4e9d2c5475a8");
    assert_md5("résumé", "a799c331edbeacb6d0f394b27aafc02a");
}

#[test]
fn md5_large_inputs() {
    assert_md5(&"a".repeat(1_000), "cabe45dcc9ae5b66ba86600cca6b8ba8");
    assert_md5(&"a".repeat(10_000), "0d0c9c4db6953fee9e03f528cafd7d3e");
    assert_md5(
        &"0123456789".repeat(1_000),
        "2bb571599a4180e1d542f76904adc3df",
    );
}

#[test]
fn md5_is_deterministic_across_repeated_calls() {
    let input = "deterministic-md5-check";
    let expected = "239604cd06ba31bd49b750fbc59ded05";

    for _ in 0..100 {
        assert_md5(input, expected);
    }
}
