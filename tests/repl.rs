mod common;
use common::expect_repl_success;
use common::expect_repl_unsuccess;

#[test]
fn exec_repl_helloworld() -> Result<(), ()> {
    expect_repl_success(
        ["print! \"hello, world\"", "exit()"]
            .into_iter()
            .map(|x| x.to_string())
            .collect(),
    )
}

#[test]
fn exec_repl_def_func() -> Result<(), ()> {
    expect_repl_success(
        [
            "f i =",
            "i + 1",
            "",
            "x = f 2",
            "assert x == 3",
            "x == 3",
            ":exit",
        ]
        .into_iter()
        .map(|line| line.to_string())
        .collect(),
    )
}

#[test]
fn exec_repl_def_loop() -> Result<(), ()> {
    expect_repl_success(
        ["for! 0..1, i =>", "print! i", "", ":exit"]
            .into_iter()
            .map(|line| line.to_string())
            .collect(),
    )
}

#[test]
fn exec_repl_auto_indent_dedent_check() -> Result<(), ()> {
    expect_repl_success(
        [
            "for! 0..0, i =>",
            "for! 0..0, j =>",
            "for! 0..0, k =>",
            "for! 0..0, l =>",
            "print! \"hi\"",
            "# l indent",
            "", // dedent l
            "# k indent",
            "", // dedent k
            "# j indent",
            "", // dedent j
            "# i indent and `for!` loop finished",
            "",
            "# main",
            ":exit",
        ]
        .into_iter()
        .map(|line| line.to_string())
        .collect(),
    )
}

#[test]
fn exec_repl_check_exit() -> Result<(), ()> {
    expect_repl_success(["", ""].into_iter().map(|x| x.to_string()).collect())
}

#[test]
fn exec_repl_invalid_indent() -> Result<(), ()> {
    expect_repl_unsuccess(
        [
            "a =",
            "    1",
            "2",
            "",
            "x =>",
            "1",
            "    print! \"hi\"",
            "",
            ":quit",
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect(),
        3,
    )
}
