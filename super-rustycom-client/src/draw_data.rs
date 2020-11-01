use std::collections::VecDeque;

pub struct DrawData {
    disassembled_history: VecDeque<String>,
    // Emulation headroom
    pub extra_nanos: u128,
    // Emulation lag
    pub missing_nanos: u128,
}

impl DrawData {
    pub fn new() -> DrawData {
        DrawData {
            disassembled_history: VecDeque::new(),
            extra_nanos: 0,
            missing_nanos: 0,
        }
    }

    pub fn disassembled_history(&self) -> &VecDeque<String> {
        &self.disassembled_history
    }

    pub fn update_history(&mut self, new_disassembly: Vec<String>, history_window: usize) {
        self.disassembled_history
            .extend(new_disassembly.into_iter());
        let history_len = self.disassembled_history.len();
        if history_len > history_window {
            self.disassembled_history
                .drain(0..history_len - history_window);
        }
    }
}
