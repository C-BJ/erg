use erg_common::vis::Visibility;

use crate::ty::constructors::{array_t, kw, mono, nd_proc1, proc, proc0};
use crate::ty::typaram::TyParam;
use crate::ty::Type;
use Type::*;

use crate::context::Context;
use crate::varinfo::Mutability;
use Mutability::*;
use Visibility::*;

impl Context {
    pub(crate) fn init_py_os_mod() -> Self {
        let mut os = Context::builtin_module("os", 15);
        os.register_builtin_py_impl(
            "chdir!",
            nd_proc1(kw("path", mono("PathLike")), NoneType),
            Immutable,
            Public,
            Some("chdir"),
        );
        os.register_builtin_py_impl("getcwd!", proc0(Str), Immutable, Public, Some("getcwd"));
        os.register_builtin_py_impl(
            "getenv!",
            nd_proc1(kw("key", Str), Str),
            Immutable,
            Public,
            Some("getenv"),
        );
        os.register_builtin_py_impl(
            "listdir!",
            proc(
                vec![],
                None,
                vec![kw("path", Str)],
                array_t(Str, TyParam::erased(Nat)),
            ),
            Immutable,
            Public,
            Some("listdir"),
        );
        os.register_builtin_py_impl(
            "mkdir!",
            nd_proc1(kw("path", mono("PathLike")), NoneType),
            Immutable,
            Public,
            Some("mkdir"),
        );
        os.register_builtin_impl("name", Str, Immutable, Public);
        os.register_builtin_py_impl(
            "putenv!",
            proc(
                vec![kw("key", Str), kw("value", Str)],
                None,
                vec![],
                NoneType,
            ),
            Immutable,
            Public,
            Some("putenv"),
        );
        os.register_builtin_py_impl(
            "remove!",
            nd_proc1(kw("path", mono("PathLike")), NoneType),
            Immutable,
            Public,
            Some("remove"),
        );
        os.register_builtin_py_impl(
            "removedirs!",
            nd_proc1(kw("name", mono("PathLike")), NoneType),
            Immutable,
            Public,
            Some("removedirs"),
        );
        os.register_builtin_py_impl(
            "rename!",
            proc(
                vec![kw("src", mono("PathLike")), kw("dst", mono("PathLike"))],
                None,
                vec![],
                NoneType,
            ),
            Immutable,
            Public,
            Some("rename"),
        );
        os.register_builtin_py_impl(
            "rmdir!",
            nd_proc1(kw("path", mono("PathLike")), NoneType),
            Immutable,
            Public,
            Some("rmdir"),
        );
        if cfg!(unix) {
            os.register_builtin_py_impl(
                "uname!",
                proc0(mono("posix.UnameResult")),
                Immutable,
                Public,
                Some("uname"),
            );
        }
        // TODO
        os.register_builtin_impl("path", mono("GenericModule"), Immutable, Public);
        os
    }
}