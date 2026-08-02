// ─────────────────────────────────────────────────────────────────────────────
// Integration tests for the Visitor API.
// ─────────────────────────────────────────────────────────────────────────────

#![cfg(feature = "alloc")]

use fasteetf::*;

// A single catch-all visitor used by most tests.  Every visit method pushes a
// stringified event onto a `Vec`, so each test can assert on the exact sequence
// of events the parser emits.
#[derive(Default)]
struct EventLog {
    events: Vec<String>,
}

impl Visitor for EventLog {
    type Error = EtfError;

    fn visit_int(&mut self, value: i32) -> Result<(), Self::Error> {
        self.events.push(format!("int({value})"));
        Ok(())
    }

    fn visit_big_int(&mut self, sign: u8, digits: &[u8]) -> Result<(), Self::Error> {
        self.events
            .push(format!("big(sign={sign},digits={digits:?})"));
        Ok(())
    }

    fn visit_float(&mut self, value: f64) -> Result<(), Self::Error> {
        self.events.push(format!("float({value})"));
        Ok(())
    }

    fn visit_atom(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!(
            "atom({})",
            std::str::from_utf8(bytes).unwrap_or("<bad utf8>")
        ));
        Ok(())
    }

    fn visit_binary(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("binary({data:?})"));
        Ok(())
    }

    fn visit_bit_binary(&mut self, bits: u8, data: &[u8]) -> Result<(), Self::Error> {
        self.events
            .push(format!("bit_binary(bits={bits},data={data:?})"));
        Ok(())
    }

    fn visit_tuple_start(&mut self, arity: usize) -> Result<(), Self::Error> {
        self.events.push(format!("tuple_start(arity={arity})"));
        Ok(())
    }

    fn visit_tuple_end(&mut self) -> Result<(), Self::Error> {
        self.events.push("tuple_end".to_string());
        Ok(())
    }

    fn visit_list_start(&mut self, len: usize) -> Result<(), Self::Error> {
        self.events.push(format!("list_start(len={len})"));
        Ok(())
    }

    fn visit_list_end(&mut self) -> Result<(), Self::Error> {
        self.events.push("list_end".to_string());
        Ok(())
    }

    fn visit_improper_list_tail(&mut self) -> Result<(), Self::Error> {
        self.events.push("improper_list_tail".to_string());
        Ok(())
    }

    fn visit_improper_list_end(&mut self) -> Result<(), Self::Error> {
        self.events.push("improper_list_end".to_string());
        Ok(())
    }

    fn visit_map_start(&mut self, arity: usize) -> Result<(), Self::Error> {
        self.events.push(format!("map_start(arity={arity})"));
        Ok(())
    }

    fn visit_map_end(&mut self) -> Result<(), Self::Error> {
        self.events.push("map_end".to_string());
        Ok(())
    }

    fn visit_pid(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("pid({data:?})"));
        Ok(())
    }

    fn visit_port(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("port({data:?})"));
        Ok(())
    }

    fn visit_reference(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("ref({data:?})"));
        Ok(())
    }

    fn visit_function(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("fun({data:?})"));
        Ok(())
    }

    fn visit_record(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("record({data:?})"));
        Ok(())
    }

    fn visit_string(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.events.push(format!("string({data:?})"));
        Ok(())
    }
}

fn run_visitor(input: &[u8], events: &mut EventLog) -> Result<(), EtfError> {
    parse_etf_with_visitor(input, None, None, events, &Limits::default())
}

mod compound;
mod defaults;
mod errors;
mod opaque;
mod scalars;
mod streaming;
