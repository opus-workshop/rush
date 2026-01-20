// Command history and search
// TODO: Implement command history persistence

pub struct History {
    commands: Vec<String>,
}

impl History {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn add(&mut self, command: String) {
        self.commands.push(command);
    }

    pub fn get(&self, index: usize) -> Option<&String> {
        self.commands.get(index)
    }

    pub fn search(&self, _query: &str) -> Vec<&String> {
        // TODO: Implement fuzzy search
        Vec::new()
    }
}
