use alloc::string::ToString;
use std::env::set_password;
use std::fs::{self, File, FileType};
use std::io::{self, prelude::*, Error};
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

fn user_map() -> Vec<(u32, String)> {
    let mut content = "".to_string();
    let mut buf = [0; 1024];
    let mut um: Vec<(u32, String)> = Vec::new();
    let mut file = File::open("/etc/passwd").unwrap_or(File::open("/dev/null").unwrap());
    loop {
        let n = file.read(&mut buf).unwrap_or(0);
        if n > 0 {
            content += String::from_utf8_lossy(&(buf[..n])).as_ref();
        } else {
            break;
        }
    }
    let lines: Vec<&str> = content.split('\n').collect();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let v: Vec<&str> = line.split(':').collect();
        let uid = String::from(v[2]).parse::<u32>().unwrap_or(u32::MAX);
        um.push((uid, v[0].trim().to_string()))
    }
    um
}

fn group_map() -> Vec<(u32, String)> {
    let mut content = "".to_string();
    let mut buf = [0; 1024];
    let mut gm: Vec<(u32, String)> = Vec::new();
    let mut file = File::open("/etc/group").unwrap_or(File::open("/dev/null").unwrap());
    loop {
        let n = file.read(&mut buf).unwrap_or(0);
        if n > 0 {
            content += String::from_utf8_lossy(&(buf[..n])).as_ref();
        } else {
            break;
        }
    }
    let lines: Vec<&str> = content.split('\n').collect();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let v: Vec<&str> = line.split(':').collect();
        let gid = String::from(v[2]).parse::<u32>().unwrap_or(u32::MAX);
        gm.push((gid, v[0].to_string()))
    }
    gm
}

pub fn user_name(uid: u32) -> String {
    for u in user_map() {
        if u.0 == uid {
            return u.1;
        }
    }
    return "root".to_string();
}
pub fn group_name(gid: u32) -> String {
    for g in group_map() {
        if g.0 == gid {
            return g.1;
        }
    }
    return "root".to_string();
}
pub fn user_id(name: String) -> u32 {
    for u in user_map() {
        if u.1 == name {
            return u.0;
        }
    }
    return 0;
}
pub fn group_id(name: String) -> u32 {
    for g in group_map() {
        if g.1 == name {
            return g.0;
        }
    }
    return 0;
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
    ("sudo", do_sudo),
    ("if_test_exist", do_if_test_exist),
    ("adduser", adduser),
    ("deluser", deluser),
    ("passwd", passwd),
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
        if pos != 0 && args.as_bytes()[pos - 1] == b'>' {
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
    let uid = user_id(u.to_string());
    let gid = group_id(g.to_string());
    if uid == 0 && u != "root" {
        print_err!("chown", "invalid user");
        return;
    }
    if gid == 0 && g != "root" {
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
        let uid: u32 = user_id(args.to_string());
        if uid == 0 && args != "root" {
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
        run_cmd(cmd.as_bytes(), "");
    }
}

fn do_sudo(args: &str) {
    let i = std::env::current_uid().unwrap();
    if let Err(e) = std::env::sudo() {
        print_err!("sudo", args, e);
        return;
    }
    run_cmd(args.as_bytes(), "");
    if let Err(e) = std::env::set_current_uid(i) {
        print_err!("sudo", args, e);
    }
}

fn adduser(args: &str) {
    fn add_file(username: &str) -> io::Result<()> {
        let mut content: String = String::new();
        let mut file = File::open("/etc/passwd")?;
        file.read_to_string(&mut content)?;
        content = content.trim().to_string();
        let last_record: Vec<&str> = content
            .split('\n')
            .last()
            .unwrap_or_default()
            .split(":")
            .collect();
        if last_record.len() != 7 {
            return Err(Error::InvalidData);
        }
        let mut file = File::create("/etc/passwd")?;
        file.write_all(content.as_bytes())?;
        let text_list = [
            "\n",
            username,
            ":x:",
            &(last_record[2].parse::<u32>().unwrap_or(0) + 1).to_string(),
            ":",
            &(last_record[3].parse::<u32>().unwrap_or(0) + 1).to_string(),
            ":,,,:/home/",
            username,
            ":/bin/sh\n",
        ];
        for text in text_list {
            file.write_all(text.as_bytes())?;
        }
        content = String::new();
        let mut file = File::open("/etc/group")?;
        file.read_to_string(&mut content)?;
        content = content.trim().to_string();
        let last_record: Vec<&str> = content
            .split('\n')
            .last()
            .unwrap_or_default()
            .split(":")
            .collect();
        if last_record.len() <= 2 {
            return Err(Error::InvalidData);
        }
        let mut file = File::create("/etc/group")?;
        file.write_all(content.as_bytes())?;
        let text_list = [
            "\n",
            username,
            ":x:",
            &(last_record[2].parse::<u32>().unwrap_or(0) + 1).to_string(),
            ":\n",
        ];
        for text in text_list {
            file.write_all(text.as_bytes())?;
        }
        let home_path = "/home/".to_string() + username;
        do_mkdir(home_path.clone().as_str());
        do_chmod(("777 ".to_string() + home_path.clone().as_str()).as_str());
        do_chown(
            (username.to_string() + ":" + username + " " + home_path.clone().as_str()).as_str(),
        );
        do_chmod(("700 ".to_string() + home_path.clone().as_str()).as_str());
        Ok(())
    }

    if !args.contains(char::is_whitespace) {
        let mut uid: u32 = u32::MAX;
        for i in 0..3 {
            if args == user_name(i) {
                uid = i;
            }
        }
        if uid != u32::MAX {
            print_err!("adduser", "existing user");
            return;
        }
        if let Err(e) = add_file(args) {
            print_err!("adduser", e);
        }
    } else {
        print_err!("deluser", "too many arguments");
    }
}

fn deluser(args: &str) {
    fn del_file(username: &str) -> io::Result<()> {
        let mut content: String = String::new();
        let mut file = File::open("/etc/passwd")?;
        file.read_to_string(&mut content)?;
        content = content.trim().to_string();
        let lines: Vec<&str> = content.split('\n').collect();
        let mut new_content = String::new();
        let mut del = false;
        for line in lines {
            if line.is_empty() {
                continue;
            }
            let v: Vec<&str> = line.split(':').collect();
            if v[0] == username.to_string() {
                del = true;
                continue;
            }
            new_content += (line.to_string() + "\n").as_str();
        }
        let mut file = File::create("/etc/passwd")?;
        file.write_all(new_content.as_bytes())?;
        if !del {
            println!("deluser: user not exist");
        }
        Ok(())
    }

    if !args.contains(char::is_whitespace) {
        if let Err(e) = del_file(args) {
            print_err!("deluser", e);
        }
    } else {
        print_err!("deluser", "too many arguments");
    }
}

fn passwd(_args: &str) {
    if let Err(e) = set_password() {
        print_err!("passwd", e);
    }
}

pub fn run_cmd(line: &[u8], args: &str) {
    fn execute_file(fname: &str, args: &str) -> io::Result<()> {
        let mut file = File::execute(fname)?;
        let mut content: String = String::new();
        file.read_to_string(&mut content)?;
        let commands: Vec<&str> = content.split('\n').collect();
        for command in commands {
            run_cmd(command.as_bytes(), args);
        }
        Ok(())
    }

    let line = unsafe { core::str::from_utf8_unchecked(line) };
    let mut line = line.trim().to_string();
    if line.is_empty() || line.as_bytes()[0] == b'#' {
        return;
    }
    let argv: Vec<&str> = args.split(' ').map(|s| s.trim()).collect();
    let mut i = 0;
    for arg in argv {
        if arg.is_empty() {
            continue;
        }
        line = line.replace(("$".to_string() + i.to_string().as_str()).as_str(), arg);
        i += 1;
    }
    let commands: Vec<&str> = line.split(';').collect();
    for line_str in commands {
        let (cmd, args) = split_whitespace(line_str);
        if !cmd.is_empty() {
            if cmd.contains('/') {
                if let Err(e) = execute_file(cmd, line_str) {
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
                if File::check(("/bin/".to_string() + cmd).as_str()).is_ok() {
                    if let Err(e) = execute_file(("/bin/".to_string() + cmd).as_str(), args) {
                        println!("{}: {}", cmd, e);
                    }
                } else {
                    println!("{}: command not found", cmd);
                }
            }
        }
    }
}

fn split_whitespace(str: &str) -> (&str, &str) {
    let str = str.trim();
    str.find(char::is_whitespace)
        .map_or((str, ""), |n| (&str[..n], str[n + 1..].trim()))
}
