use std::io::{self, Read, Write};

#[derive(Clone, Debug, Default)]
pub struct Ast {
    pub body: Vec<Op>,
}

#[derive(Clone, Debug)]
pub enum Op {
    Add(isize),
    Go(isize),
    Set(Byte),
    Loop(Ast),
    Read,
    Write,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Byte(u8);

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct Address(usize);

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display="Found {} unclosed left brackets", depth)]
    UnbalancedLeftBrackets { depth: usize },

    #[fail(display="Found unbalanced right bracket at {}", index)]
    UnbalancedRightBracket { index: usize },

    #[fail(display="IO error: {}", inner)]
    Io { inner: io::Error },
}

pub type Result<T=(), E=Error> = ::std::result::Result<T, E>;

pub fn parse(source: &str) -> Result<Ast> {
    #[derive(Default)]
    struct Builder {
        root: Ast,
        loops: Vec<Ast>,
    }

    impl Builder {
        fn emit(&mut self, op: Op) -> Result {
            if let Some(body) = self.loops.last_mut() {
                body.emit(op);
            } else {
                self.root.emit(op);
            }

            Ok(())
        }

        fn begin(&mut self) {
            self.loops.push(Ast::default());
        }

        fn end(&mut self, index: usize) -> Result {
            if let Some(body) = self.loops.pop() {
                self.emit(body.into_loop())
            } else {
                Err(Error::UnbalancedRightBracket { index })
            }
        }
    }

    impl Op {
        fn merge(&self, rhs: &Self) -> Option<Self> {
            Some(match (self, rhs) {
                (Op::Add(a), Op::Add(b)) => Op::Add(a + b),

                (Op::Go(a), Op::Go(b)) => Op::Go(a + b),

                (Op::Set(a), Op::Add(b)) => Op::Set(*a + *b),

                (Op::Add(_), Op::Set(b)) => Op::Set(*b),

                (Op::Set(_), Op::Set(b)) => Op::Set(*b),

                (Op::Go(0), _) => rhs.clone(),

                (Op::Add(0), _) => rhs.clone(),

                (Op::Add(_), Op::Read) => Op::Read,

                (Op::Set(_), Op::Read) => Op::Read,

                (Op::Loop(a), Op::Loop(_)) => Op::Loop(a.clone()),

                (Op::Loop(a), b) if a.body.is_empty() => b.clone(),

                _ => return None,
            })
        }
    }

    impl Ast {
        fn emit(&mut self, mut op: Op) {
            while let Some(a) = self.body.pop() {
                if let Some(b) = a.merge(&op) {
                    op = b;
                } else {
                    self.body.push(a);
                    break;
                }
            }

            self.body.push(op);
        }

        fn into_loop(self) -> Op {
            match self.body.as_slice() {
                [Op::Add(-1)] => return Op::Set(Byte(0)),
                [Op::Add(1)] => return Op::Set(Byte(0)),

                _ => (),
            }

            Op::Loop(self)
        }
    }

    let mut builder = Builder::default();

    for (index, ch) in source.chars().enumerate() {
        match ch {
            ',' => builder.emit(Op::Read)?,
            '.' => builder.emit(Op::Write)?,
            '+' => builder.emit(Op::Add(1))?,
            '-' => builder.emit(Op::Add(-1))?,
            '>' => builder.emit(Op::Go(1))?,
            '<' => builder.emit(Op::Go(-1))?,
            '[' => builder.begin(),
            ']' => builder.end(index)?,

            _ => continue,
        }
    }

    if builder.loops.is_empty() {
        Ok(builder.root)
    } else {
        Err(Error::UnbalancedLeftBrackets { depth: builder.loops.len() })
    }
}

pub fn interpret(ast: &Ast) -> Result {
    use std::io::{stdin, stdout, StdinLock, StdoutLock};

    const MEMSIZE: usize = 32 * 1024;

    struct Env<'a> {
        mem: Vec<Byte>,
        p: Address,
        stdin: StdinLock<'a>,
        stdout: StdoutLock<'a>,
    }

    impl<'a> Env<'a> {
        fn eval(&mut self, op: &Op) -> Result {
            match op {
                Op::Add(n) => self.mem[self.p.0] += *n,

                Op::Go(n) => self.p += *n,

                Op::Set(n) => self.mem[self.p.0] = *n,

                Op::Loop(ast) => while self.mem[self.p.0].0 != 0 {
                    for op in &ast.body {
                        self.eval(op)?;
                    }
                },

                Op::Read => {
                    let Env { mem, p, stdin, .. } = self;
                    let mut buf: [u8; 1] = [0];
                    stdin.read(&mut buf)?;
                    mem[p.0] = Byte(buf[0]);
                },

                Op::Write => {
                    let Byte(c) = self.mem[self.p.0];
                    self.stdout.write(&[c])?;
                },
            }

            Ok(())
        }
    }

    let stdin = stdin();
    let stdin = stdin.lock();
    let stdout = stdout();
    let stdout = stdout.lock();
    let mem = vec![Byte(0); MEMSIZE];
    let p = Address(0);

    let mut env = Env { mem, p, stdin, stdout };

    for op in &ast.body {
        env.eval(op)?;
    }

    Ok(())
}

impl From<io::Error> for Error {
    fn from(inner: io::Error) -> Self {
        Error::Io { inner }
    }
}

impl Ast {
    pub fn into_c(self) -> String {
        use std::fmt::{self, Write};

        struct C {
            buf: String,
            tabs: usize,
        }

        impl C {
            fn indent_line(&mut self) {
                for _ in 0 .. self.tabs {
                    self.buf.push_str("    ");
                }
            }

            fn print(&mut self, ast: Ast) -> fmt::Result {
                for op in ast.body {
                    self.indent_line();

                    match op {
                        Op::Add(n) => if n > 0 {
                            write!(self.buf, "mem[p] += {};\n", n)?;
                        } else {
                            write!(self.buf, "mem[p] -= {};\n", -n)?;
                        },

                        Op::Go(n) => if n > 0 {
                            write!(self.buf, "p += {};\n", n)?;
                        } else {
                            write!(self.buf, "p -= {};\n", -n)?;
                        },

                        Op::Set(n) => {
                            write!(self.buf, "mem[p] = {};\n", n.0)?;
                        },

                        Op::Loop(body) => {
                            write!(self.buf, "while (mem[p]) {{\n")?;
                            self.tabs += 1;
                            self.print(body)?;
                            self.tabs -= 1;
                            self.indent_line();
                            write!(self.buf, "}}\n")?;
                        },

                        Op::Read => {
                            writeln!(self.buf, "mem[p] = getchar();")?;
                        },

                        Op::Write => {
                            writeln!(self.buf, "putchar(mem[p]);")?;
                        },
                    }
                }

                Ok(())
            }
        }

        let mut c = C {
            buf: String::new(),
            tabs: 0,
        };

        c.buf.push_str("#include <stdio.h>\n\n");
        c.buf.push_str("char mem[65536] = {0};\nint p = 0;\n\n");

        c.buf.push_str("int main(int argc, char **argv) {\n");
        c.tabs = 1;
        c.print(self).unwrap();
        c.tabs = 0;
        c.buf.push_str("}\n");

        c.buf
    }
}

impl Byte {
    pub fn as_i32(self) -> i32 {
        self.0 as i32
    }
}

use std::ops::*;

impl Add<Self> for Byte {
    type Output = Byte;

    fn add(self, rhs: Self) -> Self {
        Byte(self.0.wrapping_add(rhs.0))
    }
}

impl Add<isize> for Byte {
    type Output = Byte;

    fn add(mut self, rhs: isize) -> Self {
        self += rhs; self
    }
}

fn add_signed(lhs: usize, rhs: isize, max: isize) -> usize {
    let mut r = lhs as isize + rhs;
    r %= max;
    r += max;
    r %= max;
    r as usize
}

impl AddAssign<isize> for Byte {
    fn add_assign(&mut self, rhs: isize) {
        self.0 = add_signed(self.0 as usize, rhs, 0x100) as u8;
    }
}

impl AddAssign<u8> for Byte {
    fn add_assign(&mut self, rhs: u8) {
        self.0 = self.0.wrapping_add(rhs);
    }
}

impl AddAssign<isize> for Address {
    fn add_assign(&mut self, rhs: isize) {
        self.0 = add_signed(self.0, rhs, 64 * 1024);
    }
}
