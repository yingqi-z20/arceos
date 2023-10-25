use alloc::string::ToString;
use std::fs::{self, File, FileType};
use std::io::{self, prelude::*};
use std::{string::String, vec::Vec};

#[cfg(all(not(feature = "axstd"), unix))]
use std::os::unix::fs::{FileTypeExt, PermissionsExt};

macro_rules! print_err {
    ($cmd: literal, $msg: expr) => {
        println!("{}: {}", $cmd, $msg);
    };
    ($cmd: literal, $arg: expr, $err: expr) => {
        println!("{}: {}: {}", $cmd, $arg, $err);
    };
}

type CmdHandler = fn(&str);

pub fn user_name(uid: u32) -> String {
    const USER_MAP: [&str; 3] = ["root", "admin", "guest"];
    let uid_string = uid.to_string();
    let uid_str = uid_string.as_str();
    let user = USER_MAP.get(uid as usize).unwrap_or(&uid_str);
    user.to_string()
}

pub fn group_name(gid: u32) -> String {
    const GROUP_MAP: [&str; 3] = ["root", "admin", "guest"];
    let gid_string = gid.to_string();
    let gid_str = gid_string.as_str();
    let group = GROUP_MAP.get(gid as usize).unwrap_or(&gid_str);
    group.to_string()
}

const CMD_TABLE: &[(&str, CmdHandler)] = &[
    ("cat", do_cat),
    ("cd", do_cd),
    ("echo", do_echo),
    ("halt", do_halt),
    ("exit", do_exit),
    ("help", do_help),
    ("ll", do_ll),
    ("mkdir", do_mkdir),
    ("pwd", do_pwd),
    ("rm", do_rm),
    ("uname", do_uname),
    ("chmod", do_chmod),
    ("chown", do_chown),
    ("whoami", do_whoami),
    ("su", do_su),
    ("if_test_exist", do_if_test_exist),
];

fn file_type_to_char(ty: FileType) -> char {
    if ty.is_char_device() {
        'c'
    } else if ty.is_block_device() {
        'b'
    } else if ty.is_socket() {
        's'
    } else if ty.is_fifo() {
        'p'
    } else if ty.is_symlink() {
        'l'
    } else if ty.is_dir() {
        'd'
    } else if ty.is_file() {
        '-'
    } else {
        '?'
    }
}

#[rustfmt::skip]
const fn file_perm_to_rwx(mode: u32) -> [u8; 9] {
    let mut perm = [b'-'; 9];
    macro_rules! set {
        ($bit:literal, $rwx:literal) => {
            if mode & (1 << $bit) != 0 {
                perm[8 - $bit] = $rwx
            }
        };
    }

    set!(2, b'r'); set!(1, b'w'); set!(0, b'x');
    set!(5, b'r'); set!(4, b'w'); set!(3, b'x');
    set!(8, b'r'); set!(7, b'w'); set!(6, b'x');
    perm
}

fn do_ll(args: &str) {
    let current_dir = std::env::current_dir().unwrap();
    let args = if args.is_empty() {
        path_to_str!(current_dir)
    } else {
        args
    };
    let name_count = args.split_whitespace().count();

    fn show_entry_info(path: &str, entry: &str) -> io::Result<()> {
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        let file_type = metadata.file_type();
        let file_type_char = file_type_to_char(file_type);
        let rwx = file_perm_to_rwx(metadata.permissions().mode());
        let rwx = unsafe { core::str::from_utf8_unchecked(&rwx) };
        println!(
            "{}{} {:>8} {:>8} {:>8} {}",
            file_type_char,
            rwx,
            user_name(metadata.uid()),
            group_name(metadata.gid()),
            size,
            entry
        );
        Ok(())
    }

    fn list_one(name: &str, print_name: bool) -> io::Result<()> {
        let is_dir = fs::metadata(name)?.is_dir();
        if !is_dir {
            return show_entry_info(name, name);
        }

        if print_name {
            println!("{}:", name);
        }
        let mut entries = fs::read_dir(name)?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect::<Vec<_>>();
        entries.sort();

        for entry in entries {
            let entry = path_to_str!(entry);
            let path = String::from(name) + if name.ends_with('/') { "" } else { "/" } + entry;
            if let Err(e) = show_entry_info(&path, entry) {
                print_err!("ll", path, e);
            }
        }
        Ok(())
    }

    for (i, name) in args.split_whitespace().enumerate() {
        if i > 0 {
            println!();
        }
        if let Err(e) = list_one(name, name_count > 1) {
            print_err!("ll", name, e);
        }
    }
}

fn do_cat(args: &str) {
    if args.is_empty() {
        print_err!("cat", "no file specified");
        return;
    }

    fn cat_one(fname: &str) -> io::Result<()> {
        let mut buf = [0; 1024];
        let mut file = File::open(fname)?;
        loop {
            let n = file.read(&mut buf)?;
            if n > 0 {
                io::stdout().write_all(&buf[..n])?;
            } else {
                return Ok(());
            }
        }
    }

    for fname in args.split_whitespace() {
        if let Err(e) = cat_one(fname) {
            print_err!("cat", fname, e);
        }
    }
}

fn do_echo(args: &str) {
    fn echo_file(fname: &str, text_list: &[&str], append: bool) -> io::Result<()> {
        let mut content: String = String::new();
        if append {
            let mut file = File::open(fname)?;
            file.read_to_string(&mut content)?;
        }
        let mut file = File::create(fname)?;
        file.write_all(content.as_bytes())?;
        for text in text_list {
            file.write_all(text.as_bytes())?;
        }
        Ok(())
    }

    if let Some(mut pos) = args.rfind('>') {
        let mut append = false;
        if args.as_bytes()[pos - 1] == b'>' {
            pos -= 1;
            append = true;
        }
        let text_before = args[..pos].trim();
        let (fname, text_after) = split_whitespace(&args[pos + 1 + append as usize..]);
        if fname.is_empty() {
            print_err!("echo", "no file specified");
            return;
        };

        let text_list = [
            text_before,
            if !text_after.is_empty() { " " } else { "" },
            text_after,
            "\n",
        ];
        if let Err(e) = echo_file(fname, &text_list, append) {
            print_err!("echo", fname, e);
        }
    } else {
        println!("{}", args)
    }
}

fn do_mkdir(args: &str) {
    if args.is_empty() {
        print_err!("mkdir", "missing operand");
        return;
    }

    fn mkdir_one(path: &str) -> io::Result<()> {
        fs::create_dir(path)
    }

    for path in args.split_whitespace() {
        if let Err(e) = mkdir_one(path) {
            print_err!("mkdir", format_args!("cannot create directory '{path}'"), e);
        }
    }
}

fn do_rm(args: &str) {
    if args.is_empty() {
        print_err!("rm", "missing operand");
        return;
    }
    let mut rm_dir = false;
    for arg in args.split_whitespace() {
        if arg == "-d" {
            rm_dir = true;
        }
    }

    fn rm_one(path: &str, rm_dir: bool) -> io::Result<()> {
        if rm_dir && fs::metadata(path)?.is_dir() {
            fs::remove_dir(path)
        } else {
            fs::remove_file(path)
        }
    }

    for path in args.split_whitespace() {
        if path == "-d" {
            continue;
        }
        if let Err(e) = rm_one(path, rm_dir) {
            print_err!("rm", format_args!("cannot remove '{path}'"), e);
        }
    }
}

fn do_cd(mut args: &str) {
    if args.is_empty() {
        args = "/";
    }
    if !args.contains(char::is_whitespace) {
        if let Err(e) = std::env::set_current_dir(args) {
            print_err!("cd", args, e);
        }
    } else {
        print_err!("cd", "too many arguments");
    }
}

fn do_pwd(_args: &str) {
    let pwd = std::env::current_dir().unwrap();
    println!("{}", path_to_str!(pwd));
}

fn do_uname(_args: &str) {
    let arch = option_env!("AX_ARCH").unwrap_or("");
    let platform = option_env!("AX_PLATFORM").unwrap_or("");
    let smp = match option_env!("AX_SMP") {
        None | Some("1") => "",
        _ => " SMP",
    };
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.1.0");
    println!(
        "ArceOS {ver}{smp} {arch} {plat}",
        ver = version,
        smp = smp,
        arch = arch,
        plat = platform,
    );
}

fn do_help(_args: &str) {
    println!("Available commands:");
    for (name, _) in CMD_TABLE {
        println!("  {}", name);
    }
}

fn do_halt(_args: &str) {
    std::process::halt(0);
    print_err!("halt", "Permission denied");
}

fn do_exit(args: &str) {
    println!("Bye~");
    std::process::exit(if args.is_empty() {
        0
    } else {
        (args.as_bytes()[0] - b'0') as i32
    });
}

pub fn run_cmd(line: &[u8]) {
    fn execute_file(fname: &str, _args: &str) -> io::Result<()> {
        let mut file = File::execute(fname)?;
        let mut content: String = String::new();
        file.read_to_string(&mut content)?;
        let commands: Vec<&str> = content.split('\n').collect();
        for command in commands {
            run_cmd(command.as_bytes());
        }
        Ok(())
    }

    let line = unsafe { core::str::from_utf8_unchecked(line) };
    let commands: Vec<&str> = line.split(';').collect();
    for line_str in commands {
        let (cmd, args) = split_whitespace(line_str);
        if !cmd.is_empty() {
            if cmd.contains('/') {
                if let Err(e) = execute_file(cmd, args) {
                    println!("{}: {}", cmd, e);
                }
                continue;
            }
            let mut inner_cmd = false;
            for (name, func) in CMD_TABLE {
                if cmd == *name {
                    func(args);
                    inner_cmd = true;
                    break;
                }
            }
            if !inner_cmd {
                println!("{}: command not found", cmd);
            }
        }
    }
}

fn split_whitespace(str: &str) -> (&str, &str) {
    let str = str.trim();
    str.find(char::is_whitespace)
        .map_or((str, ""), |n| (&str[..n], str[n + 1..].trim()))
}

fn do_chmod(args: &str) {
    let mut recursive = false;
    let mut mf = Vec::new();
    for arg in args.split_whitespace() {
        if arg == "-R" {
            recursive = true;
        } else {
            mf.push(arg)
        }
    }
    if mf.len() != 2 {
        print_err!("chmod", "wrong operand");
        return;
    }
    if recursive {
        print_err!("chmod", "can't recursive chmod yet");
    }
    let m = mf[0].as_bytes();
    let fname = mf[1];

    fn chmod_one(fname: &str, perm: u16) -> io::Result<()> {
        let file = File::check(fname)?;
        let md = file.metadata().unwrap();
        file.change_metadata(perm, md.uid(), md.gid())
    }

    let perm = (m[0] - b'0') as u16 * 64 + (m[1] - b'0') as u16 * 8 + (m[2] - b'0') as u16;
    if let Err(e) = chmod_one(fname, perm) {
        print_err!("chmod", fname, e);
    }
}

fn do_chown(args: &str) {
    let mut recursive = false;
    let mut mf = Vec::new();
    for arg in args.split_whitespace() {
        if arg == "-R" {
            recursive = true;
        } else {
            mf.push(arg)
        }
    }
    if mf.len() != 2 {
        print_err!("chown", "wrong operand");
        return;
    }
    if recursive {
        print_err!("chown", "can't recursive chown yet");
    }
    let ug = mf[0];
    let fname = mf[1];
    let (u, g) = ug
        .find(':')
        .map_or((ug, ""), |n| (&ug[..n], ug[n + 1..].trim()));
    let mut uid: u32 = u32::MAX;
    let mut gid: u32 = u32::MAX;
    for i in 0..3 {
        if u == user_name(i) {
            uid = i;
        }
    }
    for i in 0..3 {
        if g == group_name(i) {
            gid = i;
        }
    }
    if uid == u32::MAX {
        print_err!("chown", "invalid user");
        return;
    }
    if gid == u32::MAX {
        print_err!("chown", "invalid group");
        return;
    }

    fn chown_one(fname: &str, uid: u32, gid: u32) -> io::Result<()> {
        let file = File::check(fname)?;
        let md = file.metadata().unwrap();
        file.change_metadata(md.permissions().mode() as u16, uid, gid)
    }

    if let Err(e) = chown_one(fname, uid, gid) {
        print_err!("chown", fname, e);
    }
}

fn do_whoami(_args: &str) {
    let i = std::env::current_uid().unwrap();
    println!("{}", user_name(i));
}

fn do_su(args: &str) {
    if !args.contains(char::is_whitespace) {
        let mut uid: u32 = u32::MAX;
        for i in 0..3 {
            if args == user_name(i) {
                uid = i as u32;
            }
        }
        if uid == u32::MAX {
            print_err!("su", "invalid user");
            return;
        }
        if let Err(e) = std::env::set_current_uid(uid) {
            print_err!("su", args, e);
        }
    } else {
        print_err!("su", "too many arguments");
    }
}

fn do_if_test_exist(args: &str) {
    let (fname, cmd) = split_whitespace(args);
    if File::check(fname).is_ok() {
        run_cmd(cmd.as_bytes());
    }
}
