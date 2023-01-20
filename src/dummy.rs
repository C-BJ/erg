use std::fs::remove_file;
use std::io::{stdout, BufWriter, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::thread::sleep;
use std::time::Duration;
use std::{mem, process};

use erg_common::config::{ErgConfig, Input};
use erg_common::error::{ErrorDisplay, ErrorKind, MultiErrorDisplay};
use erg_common::python_util::{exec_pyc, spawn_py};
use erg_common::traits::{BlockKind, Runnable, Stream, VirtualMachine};
use erg_common::{chomp, log};

use erg_compiler::hir::Expr;
use erg_compiler::ty::HasType;

use erg_compiler::error::{CompileError, CompileErrors};
use erg_compiler::Compiler;

pub type EvalError = CompileError;
pub type EvalErrors = CompileErrors;

/// Open the Python interpreter as a server and act as an Erg interpreter by mediating communication
///
/// Pythonインタープリタをサーバーとして開き、通信を仲介することでErgインタープリタとして振る舞う
#[derive(Debug)]
pub struct DummyVM {
    compiler: Compiler,
    stream: Option<TcpStream>,
}

impl Default for DummyVM {
    fn default() -> Self {
        Self::new(ErgConfig::default())
    }
}

impl Drop for DummyVM {
    fn drop(&mut self) {
        self.finish();
    }
}

impl Runnable for DummyVM {
    type Err = EvalError;
    type Errs = EvalErrors;
    const NAME: &'static str = "Erg interpreter";

    #[inline]
    fn cfg(&self) -> &ErgConfig {
        &self.compiler.cfg
    }
    #[inline]
    fn cfg_mut(&mut self) -> &mut ErgConfig {
        &mut self.compiler.cfg
    }

    fn new(cfg: ErgConfig) -> Self {
        let stream = if cfg.input.is_repl() {
            if !cfg.quiet_repl {
                println!("Starting the REPL server...");
            }
            let port = find_available_port();
            let code = include_str!("scripts/repl_server.py")
                .replace("__PORT__", port.to_string().as_str());
            spawn_py(cfg.py_command, &code);
            let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
            if !cfg.quiet_repl {
                println!("Connecting to the REPL server...");
            }
            loop {
                match TcpStream::connect(addr) {
                    Ok(stream) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(cfg.py_server_timeout)))
                            .unwrap();
                        break Some(stream);
                    }
                    Err(_) => {
                        if !cfg.quiet_repl {
                            println!("Retrying to connect to the REPL server...");
                        }
                        sleep(Duration::from_millis(500));
                        continue;
                    }
                }
            }
        } else {
            None
        };
        Self {
            compiler: Compiler::new(cfg),
            stream,
        }
    }

    fn finish(&mut self) {
        if let Some(stream) = &mut self.stream {
            if let Err(err) = stream.write_all("exit".as_bytes()) {
                eprintln!("Write error: {err}");
                process::exit(1);
            }
            let mut buf = [0; 1024];
            match stream.read(&mut buf) {
                Result::Ok(n) => {
                    let s = std::str::from_utf8(&buf[..n]).unwrap();
                    if s.contains("closed") && !self.cfg().quiet_repl {
                        println!("The REPL server is closed.");
                    }
                }
                Result::Err(err) => {
                    eprintln!("Read error: {err}");
                    process::exit(1);
                }
            }
            remove_file("o.pyc").unwrap_or(());
        }
    }

    fn initialize(&mut self) {
        self.compiler.initialize();
    }

    fn clear(&mut self) {
        self.compiler.clear();
    }

    fn exec(&mut self) -> Result<i32, Self::Errs> {
        // Parallel execution is not possible without dumping with a unique file name.
        let filename = self.cfg().dump_pyc_filename();
        let src = self.cfg_mut().input.read();
        let warns = self
            .compiler
            .compile_and_dump_as_pyc(&filename, src, "exec")
            .map_err(|eart| {
                eart.warns.fmt_all_stderr();
                eart.errors
            })?;
        warns.fmt_all_stderr();
        let code = exec_pyc(&filename, self.cfg().py_command, &self.cfg().runtime_args);
        remove_file(&filename).unwrap();
        Ok(code.unwrap_or(1))
    }

    #[inline]
    fn expect_block(&self, src: &str) -> BlockKind {
        let multi_line_str = "\"\"\"";
        if src.contains(multi_line_str) && src.rfind(multi_line_str) == src.find(multi_line_str) {
            return BlockKind::MultiLineStr;
        }
        if src.ends_with("do!:") && !src.starts_with("do!:") {
            return BlockKind::Lambda;
        }
        if src.ends_with("do:") && !src.starts_with("do:") {
            return BlockKind::Lambda;
        }
        if src.ends_with(':') && !src.starts_with(':') {
            return BlockKind::Lambda;
        }
        if src.ends_with('=') && !src.starts_with('=') {
            return BlockKind::Assignment;
        }
        if src.ends_with('.') && !src.starts_with('.') {
            return BlockKind::ClassPub;
        }
        if src.ends_with("::") && !src.starts_with("::") {
            return BlockKind::ClassPriv;
        }
        if src.ends_with("=>") && !src.starts_with("=>") {
            return BlockKind::Lambda;
        }
        if src.ends_with("->") && !src.starts_with("->") {
            return BlockKind::Lambda;
        }

        BlockKind::None
    }

    fn eval(&mut self, src: String) -> Result<String, EvalErrors> {
        let arti = self
            .compiler
            .eval_compile_and_dump_as_pyc("o.pyc", src, "eval")
            .map_err(|eart| eart.errors)?;
        let (last, warns) = (arti.object, arti.warns);
        let mut res = warns.to_string();
        // Tell the REPL server to execute the code
        res += &match self.stream.as_mut().unwrap().write("load".as_bytes()) {
            Result::Ok(_) => {
                // read the result from the REPL server
                let mut buf = [0; 1024];
                match self.stream.as_mut().unwrap().read(&mut buf) {
                    Result::Ok(n) => {
                        let s = std::str::from_utf8(&buf[..n])
                            .expect("failed to parse the response, maybe the output is too long");
                        if s == "[Exception] SystemExit" {
                            return Err(EvalErrors::from(EvalError::system_exit()));
                        }
                        s.to_string()
                    }
                    Result::Err(err) => {
                        self.finish();
                        eprintln!("Read error: {err}");
                        process::exit(1);
                    }
                }
            }
            Result::Err(err) => {
                self.finish();
                eprintln!("Sending error: {err}");
                process::exit(1);
            }
        };
        if res.ends_with("None") {
            res.truncate(res.len() - 5);
        }
        if self.cfg().show_type {
            res.push_str(": ");
            res.push_str(
                &last
                    .as_ref()
                    .map(|last| last.t())
                    .unwrap_or_default()
                    .to_string(),
            );
            if let Some(Expr::Def(def)) = last {
                res.push_str(&format!(" ({})", def.sig.ident()));
            }
        }
        Ok(res)
    }
}

impl DummyVM {
    /// Execute the script specified in the configuration.
    pub fn exec(&mut self) -> Result<i32, EvalErrors> {
        Runnable::exec(self)
    }

    /// Evaluates code passed as a string.
    pub fn eval(&mut self, src: String) -> Result<String, EvalErrors> {
        Runnable::eval(self, src)
    }

    /// for the tests
    /// same as a run() of trait.rs but return CompileErr
    pub fn dummy_run(cfg: ErgConfig) -> Result<i32, CompileErrors> {
        let quiet_repl = cfg.quiet_repl;
        let mut instance = Self::new(cfg);
        let mut all_errors = CompileErrors::new(vec![]);
        match instance.input() {
            Input::File(_) | Input::Pipe(_) | Input::Str(_) | Input::REPL | Input::Dummy => {
                unreachable!()
            }
            Input::DummyREPL(_) => {
                let output = stdout();
                let mut output = BufWriter::new(output.lock());
                if !quiet_repl {
                    log!(info_f output, "The REPL has started.\n");
                    output
                        .write_all(instance.start_message().as_bytes())
                        .unwrap();
                }
                let mut vm = VirtualMachine::new();
                loop {
                    let indent = vm.indent();
                    if vm.length > 1 {
                        output.write_all(instance.ps2().as_bytes()).unwrap();
                        output.write_all(indent.as_str().as_bytes()).unwrap();
                        output.flush().unwrap();
                    } else {
                        output.write_all(instance.ps1().as_bytes()).unwrap();
                        output.flush().unwrap();
                    }
                    instance.cfg().input.set_indent(vm.length);
                    let line = chomp(&instance.cfg_mut().input.read());
                    let line = line.trim_end();
                    match line {
                        ":quit" | ":exit" => {
                            break;
                        }
                        "@Inheritable" | "@Override" => {
                            vm.push_code(line);
                            vm.push_code("\n");
                            vm.push_block_kind(BlockKind::AtMark);
                            continue;
                        }
                        "" => {
                            // eval after the end of the block
                            if vm.length == 2 {
                                vm.remove_block_kind();
                            } else if vm.length > 1 {
                                vm.remove_block_kind();
                                vm.push_code("\n");
                                continue;
                            }
                            match instance.eval(mem::take(&mut vm.codes)) {
                                Ok(out) if out.is_empty() => continue,
                                Ok(out) => {
                                    output.write_all((out + "\n").as_bytes()).unwrap();
                                    output.flush().unwrap();
                                }
                                Err(errs) => {
                                    if errs
                                        .first()
                                        .map(|e| e.core().kind == ErrorKind::SystemExit)
                                        .unwrap_or(false)
                                    {
                                        break;
                                    }
                                    errs.fmt_all_stderr();
                                    all_errors.extend(errs.into_iter());
                                }
                            }
                            instance.input().set_block_begin();
                            instance.clear();
                            vm.clear();
                            continue;
                        }
                        _ => {}
                    }
                    let line = if let Some(comment_start) = line.find('#') {
                        &line[..comment_start]
                    } else {
                        line
                    };
                    let bk = instance.expect_block(line);
                    match bk {
                        BlockKind::None => {
                            if vm.now == BlockKind::MultiLineStr {
                                vm.push_code(line);
                                vm.push_code("\n");
                                continue;
                            }
                            vm.push_code(indent.as_str());
                            instance.input().insert_whitespace(indent.as_str());
                            vm.push_code(line);
                            vm.push_code("\n");
                        }
                        BlockKind::Error => {
                            vm.push_code(indent.as_str());
                            instance.input().insert_whitespace(indent.as_str());
                            vm.push_code(line);
                            vm.now = BlockKind::Main;
                            vm.now_block = vec![BlockKind::Main];
                        }
                        BlockKind::MultiLineStr => {
                            // end of MultiLineStr
                            if vm.now == BlockKind::MultiLineStr {
                                vm.remove_block_kind();
                                vm.push_code(line);
                                vm.push_code("\n");
                            } else {
                                // start of MultiLineStr
                                vm.push_block_kind(BlockKind::MultiLineStr);
                                vm.push_code(indent.as_str());
                                instance.input().insert_whitespace(indent.as_str());
                                vm.push_code(line);
                                vm.push_code("\n");
                                continue;
                            }
                        }
                        // expect block
                        _ => {
                            if vm.length == 1 {
                                instance.input().set_block_begin();
                            }
                            // even if the parser expects a block, line will all be string
                            if vm.now != BlockKind::MultiLineStr {
                                vm.push_code(indent.as_str());
                                instance.input().insert_whitespace(indent.as_str());
                                vm.push_block_kind(bk);
                            }
                            vm.push_code(line);
                            vm.push_code("\n");
                            continue;
                        }
                    } // single eval
                    if vm.now == BlockKind::Main {
                        match instance.eval(mem::take(&mut vm.codes)) {
                            Ok(out) => {
                                output.write_all((out + "\n").as_bytes()).unwrap();
                                output.flush().unwrap();
                            }
                            Err(errs) => {
                                if errs
                                    .first()
                                    .map(|e| e.core().kind == ErrorKind::SystemExit)
                                    .unwrap_or(false)
                                {
                                    break;
                                }
                                errs.fmt_all_stderr();
                                all_errors.extend(errs.into_iter());
                            }
                        }
                        instance.input().set_block_begin();
                        instance.clear();
                        vm.clear();
                    }
                }
            }
        }
        if all_errors.is_empty() {
            Ok(0)
        } else {
            Err(all_errors)
        }
    }
}

fn find_available_port() -> u16 {
    const DEFAULT_PORT: u16 = 8736;
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, DEFAULT_PORT))
        .is_ok()
        .then_some(DEFAULT_PORT)
        .unwrap_or_else(|| {
            let socket = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
            TcpListener::bind(socket)
                .and_then(|listener| listener.local_addr())
                .map(|sock_addr| sock_addr.port())
                .expect("No free port found.")
        })
}
