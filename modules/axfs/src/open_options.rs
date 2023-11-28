/// Options and flags which can be used to configure how a file is opened.
#[derive(Clone)]
pub struct OpenOptions {
    // generic
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub append: bool,
    pub truncate: bool,
    pub create: bool,
    pub create_new: bool,
    // system-specific
    pub _custom_flags: i32,
    pub _mode: u32,
}

impl OpenOptions {
    /// Creates a blank new set of options ready for configuration.
    pub const fn new() -> Self {
        Self {
            // generic
            read: false,
            write: false,
            execute: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
            // system-specific
            _custom_flags: 0,
            _mode: 0o666,
        }
    }
    /// Sets the option for read access.
    pub fn read(&mut self, read: bool) {
        self.read = read;
    }
    /// Sets the option for write access.
    pub fn write(&mut self, write: bool) {
        self.write = write;
    }
    /// Sets the option for execute access.
    pub fn execute(&mut self, execute: bool) {
        self.execute = execute;
    }
    /// Sets the option for the append mode.
    pub fn append(&mut self, append: bool) {
        self.append = append;
    }
    /// Sets the option for truncating a previous file.
    pub fn truncate(&mut self, truncate: bool) {
        self.truncate = truncate;
    }
    /// Sets the option to create a new file, or open it if it already exists.
    pub fn create(&mut self, create: bool) {
        self.create = create;
    }
    /// Sets the option to create a new file, failing if it already exists.
    pub fn create_new(&mut self, create_new: bool) {
        self.create_new = create_new;
    }
}
