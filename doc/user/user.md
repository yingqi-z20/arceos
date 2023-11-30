# 用户权限管理模块

**张英奇 2023-11-23**

本项目实现了 ArceOS 上的一个用户权限管理模块，支持与 linux 功能基本一致的`halt`、`exit`、`ll`、`chmod`、`chown`、`whoami`、`su`、`sudo`、`adduser`、`deluser`、`passwd`等命令。

相较于展示的ppt，本文档更关注具体的实现和使用方法。

## 目录

1.   文件权限管理
2.   进程权限管理
3.   用户信息管理
4.   依赖关系与模块化

## 文件权限管理

### 阻止无权限用户访问

```rust
pub fn fops_cap(perm: VfsNodePerm, uid: u32, gid: u32) -> Cap
```

根据文件权限模式、文件所有者和当前用户，给出用户是否拥有对此文件的相应权限，权限不足时系统调用返回`PermissionDenied`错误。

当一个文件具有x属性时，可以作为一个shell脚本执行，即将其内容作为shell的输入来运行；利用这一点，加上条件执行命令，可以构成图灵完备的shell程序语言。

### chmod 和 chown

![](https://www.runoob.com/wp-content/uploads/2014/08/rwx-standard-unix-permission-bits.png)

chmod命令使用八进制数来指定文件访问权限模式，`chown user:group file`指定文件所有者和所有组，二者都是通过系统调用`ax_file_change_attr`实现，`ax_file_change_attr`在文件属于当前用户的情况下通过文件系统提供的接口`set_attr`修改文件元信息，否则返回`PermissionDenied`错误。

```rust
pub fn ax_file_change_attr(file: &AxFileHandle, perm: u16, uid: u32, gid: u32) -> AxResult
```

但是在fat文件系统中，`set_attr`被实现为无作用的，因此当前此命令只在ramfs上有效。

### ll 命令与文件夹权限

为了符合linux系统的情况，我将ls命令改为了ll，可以列出此目录下文件访问权限模式、文件所属的用户和组、文件大小和名称。

在中期以期，我曾错误地认为若文件对当前用户无读权限，则用户也不能够读出文件元信息。但是linux系统中，若文件所在的文件夹可读，文件元信息就可读。此外文件夹w权限约束用户对文件夹下文件和目录的添加/删除。在文件夹对用户无x权限时，此用户不可进入此文件夹，也就是不能cd到此目录下。

为支持使用ll命令获取不可读文件的元信息，我允许无权限请求地调用open，此时返回的文件描述符只允许查看文件元信息。

## 进程权限管理

### 进程所有者

在axfs中，进程所在目录是一个全局变量：

```rust
static CURRENT_DIR_PATH: Mutex<String> = Mutex::new(String::new());
static CURRENT_DIR: LazyInit<Mutex<VfsNodeRef>> = LazyInit::new();
```

这是很不合理的，表现了unikernel去掉PCB引发的一些数据存放的问题。进程所有者的保存也是类似的问题，只考虑单进程的情况下，我也采用了这种设计：

```rust
static UID: Mutex<u32> = Mutex::new(0);
```

进程具有一位所有者，进程所能执行的操作取决于这位所有者拥有的权限。

whoami命令可以查看当前进程所有用户的名称。

新建的文件和文件夹，所有者为当前用户。

### 用户登录和切换

```rust
pub fn ax_setuid(uid: u32) -> AxResult
```

切换用户使用su命令，对应的系统调用是`ax_setuid`，仅在使用su命令并输入正确口令才允许切换为其他用户身份。在切换用户和登录时，由内核接管标准输入输出，输入的口令将不会显示出来。

在我的设计中，exit命令只会退出当前用户，不会关闭机器，而是进入重新登录状态，需输入用户名和密码；只有以root权限执行halt命令才能关闭机器。

`sudo`系统调用相当于使用sudoer自己的密码验证并添加sudoers验证的setuid(0)，执行后需要用户程序（sudo命令）自行返回到原用户。

### 用户权限等级

用户分为三类：root用户，sudoers和普通用户：

1.   root用户可以执行任何命令而不受权限限制（只有0号用户root）；
2.   sudoers可以使用sudo和自己的口令暂时取得和root用户相同的权限，但不使用sudo时与普通用户相同；
3.   普通用户没有特殊的权限，也不可以通过sudo暂时提权。

## 用户信息管理

*执行持久化存储的磁盘镜像中的`/root/init.sh`，可以初始化各用户信息配置文件。`init.sh`内容如下*

```sh
cd /
echo root:x:0:0:root:/root:/bin/bash > /etc/passwd
echo root:x:0: > /etc/group
echo 0:hmekinajmkdhgckpgbofjfcajednmcgejepijebl > /etc/shadow
echo root ALL=(ALL:ALL) ALL > /etc/sudoers
chmod 600 /etc/shadow
chmod 600 /etc/sudoers
exit
```

### 用户公开信息管理

`/etc/passwd`、`/etc/group`对所有用户可读但不可写，`/etc/shadow`、`/etc/sudoers`仅root权限可访问。

shell和内核总是以最新的`/etc/passwd`和`/etc/group`为准，查询用户、UID、组、GID间的对应关系。

### 密码管理

已知旧密码，可以通过passwd命令修改自己的密码，由内核接管标准输入输出，输入的新旧密码将不会显示出来。

密码使用SHA1哈希后存储在`/etc/shadow`中，没有明文保存的密码。

### 用户添加和删除

实现了adduser和deluser命令，除root以外的用户都可以动态变化。

添加用户时，在/home目录下生成仅能由此用户读写的主目录；为确保数据不会遗失，删除用户时此目录不会随之删除。

### sudoers动态管理

在使用sudo命令时，内核基于`/etc/sudoers`判断此用户是否拥有sudo权限，并拒绝不在sudoers中的用户提权。

完善了echo的功能，支持使用`>>`追加。

## 依赖关系与模块化

### 依赖关系

由于用户配置文件具有特定的权限，要对其访问进行控制，axfs的文件操作就必须支持文件权限管理和查询进程所有者。由此可见，用户信息管理依赖于文件系统模块，而文件系统模块又依赖于文件权限管理和进程权限管理。这使得将用户权限管理系统实现为一个单独的模块是非常困难的，因为这会与文件系统模块存在交叉的依赖关系。

原本的模块化设计是将用户管理作为axfs的一个feature，但这样就有比较高的耦合性。最终我将用户权限管理系统分为两个模块：通用的`crates/permission`和可选的`modules/axuser`。

### crates/permission

负责文件权限管理和进程权限管理，提供通用的文件权限和进程所有者操作，可以作为axfs的一个feature被引入。

### modules/axuser

负责用户信息管理，依赖axfs和crates/permission；为系统调用层提供密码验证、sudoers查询、用户id查找等api。

### 模块开启/关闭

在`apps/fs/shell/Cargo.toml`中，改变`[features]`的`default`可以控制是否开启用户管理模块；此处若存在`"user"`，则crates/permission和modules/axuser会被加载。

```toml
[features]
use-ramfs = ["axstd/myfs", "dep:axfs_vfs", "dep:axfs_ramfs", "dep:crate_interface"]
user = ["axstd/user"]
default = ["user"]
```

若模块关闭，所有指令不受权限约束，并且动态用户信息管理失效，相当于始终以root用户执行操作。

若模块开启，axfs会打开permission的feature。以实现对用户访问文件系统的约束；系统调用可以对接到modules/axuser，使其具备实际功能。

未在`apps/fs/shell/Cargo.toml`使用 default = ["user"] 时，也可以使用如下命令开启用户管理模块

```shell
make A=apps/fs/shell FEATURES=user ARCH=riscv64 LOG=error BLK=y run
```

