use parity_wasm::elements::Instruction as Instr;

use bf::{Ast, Op};

pub enum Error {

}

pub type Result<T=(), E=Error> = ::std::result::Result<T, E>;

impl Ast {
    pub fn into_wasm(self) -> Result<Vec<u8>> {
        let mut builder = Builder {
            buf: vec![],
        };

        builder.translate(self)?;

        builder.finish()
    }
}

struct Builder {
    buf: Vec<Instr>,
}

const MEM: u32 = 0;
const P: u32 = 1;

impl Builder {
    fn translate(&mut self, ast: Ast) -> Result {
        for op in ast.body {
            match op {
                Op::Add(n) => {
                    self.read_tape()?;
                    self.emit(Instr::I32Const(n as i32))?;
                    self.emit(Instr::I32Add)?;
                    self.write_tape()?;
                },

                Op::Go(n) => {
                    self.read_ptr()?;
                    self.emit(Instr::I32Const(n as i32))?;
                    self.emit(Instr::I32Add)?;
                    self.write_ptr()?;
                },

                Op::Set(n) => {
                    self.emit(Instr::I32Const(n.as_i32()))?;
                    self.write_tape()?;
                },

                Op::Loop(body) => {
                    use parity_wasm::elements::BlockType;

                    // FIXME: Branch to end if zero
                    self.emit(Instr::Loop(BlockType::NoResult))?;

                    self.translate(body)?;

                    // FIXME: Branch to loop if nonzero
                    self.emit(Instr::End)?;
                },

                Op::Read => {
                    // FIXME: Read input
                    self.write_tape()?;
                },

                Op::Write => {
                    self.read_tape()?;
                    // FIXME: Write output
                },
            }
        }

        Ok(())
    }

    fn read_mem(&mut self) -> Result {
        self.emit(Instr::GetGlobal(MEM))
    }

    fn read_ptr(&mut self) -> Result {
        self.emit(Instr::GetGlobal(P))
    }

    fn write_ptr(&mut self) -> Result {
        self.emit(Instr::SetGlobal(P))
    }

    fn read_tape(&mut self) -> Result {
        self.read_mem()?;
        self.read_ptr()?;
        self.emit(Instr::I32Add)?;
        self.emit(Instr::I32Load8U(0, 0))
    }

    fn write_tape(&mut self) -> Result {
        self.read_mem()?;
        self.read_ptr()?;
        self.emit(Instr::I32Add)?;
        self.emit(Instr::I32Store8(0, 0))
    }

    fn emit(&mut self, i: Instr) -> Result {
        self.buf.push(i);

        Ok(())
    }

    fn finish(self) -> Result<Vec<u8>> {
        // FIXME: Module prelude etc.
        // FIXME: Actually encode thing
        Ok(vec![])
    }
}
