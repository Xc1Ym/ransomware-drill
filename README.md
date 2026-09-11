# 勒索病毒应急演练演示套件

一套用于**网络安全应急演练**的演示工具，让参演人员直观看到勒索攻击发生时的真实观感，
再演练应急处置与恢复流程。

支持 macOS 与 Windows，一次配置、重新编译即可整体换皮，适配不同单位的演练需求。

> ## ⚠️ 这是演练道具，不是勒索软件
>
> 本套件**不具备任何真实的加密、破坏或传播能力**：
>
> - **只改文件名**——给文件追加一个后缀，文件内容一个字节都不会动；
> - **不加密**——代码里没有任何密码学运算；
> - **不删除**——代码里不存在删除文件的路径；
> - **不联网**——没有任何网络代码，弹窗上的邮箱与比特币地址均为虚构；
> - **不驻留**——窗口关掉进程就没了，不会开机自启、不会后台常驻；
> - **可完整还原**——随附的 `drill-restorer` 能 100% 还原文件名与桌面壁纸。
>
> 请仅在获得授权的演练场景中使用。

---

## 换一个单位演练？只需要三步

```bash
# 1. 改配置：把单位名称、文案、邮箱、金额改成目标单位的
vim config/drill.toml

# 2. 重新打包（会自动编译 + 渲染网页 + 生成壁纸）
cargo run -p xtask -- package

# 3. 把生成的 dist/ 整个目录拷到演练机器上
```

`dist/` 里的所有东西（弹窗标题与正文、壁纸标语、下载页上的单位名、恢复工具的报告抬头）
都会同步变成新单位的信息——它们读的是同一份配置。

---

## 目录结构

```
ransom/
├── config/drill.toml      ★ 唯一配置源，换单位只改这里
├── crates/
│   ├── drill-core/            核心库：配置、锁定/恢复、壁纸渲染、平台适配
│   ├── drill-locker/          演练端：改后缀 + 换壁纸 + 弹窗
│   ├── drill-restorer/        恢复端：还原后缀与壁纸
│   ├── drill-webgen/          网页生成
│   └── xtask/                 一键打包入口
├── web/
│   ├── index.html             仿浏览器新标签页（钓鱼入口，搜索框可输入）
│   ├── search.html            仿搜索引擎结果页（搜索结果里的「广告」位是陷阱链接）
│   └── download.html          仿软件下载站（点击下载得到演练程序）
└── dist/                      打包产物（不入库，由 xtask 生成）
```

---

## 快速开始

```bash
# 编译
cargo build --release

# 造一批测试样本文件
cargo run -p xtask -- sandbox

# 先预览会影响哪些文件（不会做任何改动）
./target/release/drill-locker --root target/drill-sandbox --dry-run

# 实际执行演练
./target/release/drill-locker --root target/drill-sandbox

# 恢复
./target/release/drill-restorer --root target/drill-sandbox --yes
```

---

## 演练流程

1. **准备**：把 `dist/` 拷到演练机器上一个**专用文件夹**（例如桌面上的 `演练演示/`）。
   ⚠️ 不要放在家目录、桌面根目录或任何系统目录下——程序会拒绝执行。
2. **展示钓鱼链路**：打开 `dist/index.html`，在最上方的搜索框里输入关键词（例如 `chrome 下载`）
   回车，进入仿搜索结果页；点击置顶的那条「广告」结果，会进入一个下载页。
   这条链路演示的正是「搜到啥点啥 → 从仿冒站点下载 → 中招」这一最常见的中招路径。
   - 搜浏览器类关键词时，结果页会把陷阱伪装成「浏览器官方下载」；
     搜其它关键词时，则显示配置里 `web.download_name` 对应的软件。
   - 陷阱链接展示的仿冒域名由 `web.fake_domain` 配置（默认 `downlod-center.com`）。
3. **触发演练**：在下载页点「立即下载」，或直接运行 `drill-locker`
   （macOS 双击 `start-drill.command`，Windows 双击 `start-drill.bat`）。
   程序会锁定所在目录下的文件、更换壁纸、弹出勒索窗口。
4. **观察响应**：记录参演人员的发现时间、上报流程、处置动作。

## 恢复流程

```bash
# 先看看会恢复什么
./drill-restorer --dry-run

# 执行恢复
./drill-restorer --yes
```

或直接双击 `restore.command`（macOS）/ `restore.bat`（Windows）。

恢复工具会做两件事：还原全部文件名后缀、把壁纸设回演练前的图片。

---

## 安全护栏

工具内置了多层保护，防止演练变成事故：

| 护栏 | 说明 |
|---|---|
| **目录黑名单** | 拒绝在 `/`、`/System`、`/Library`、`C:\Windows`、`C:\Program Files` 等系统目录及其任何子目录下执行 |
| **用户目录保护** | 拒绝把家目录本身、`~/Desktop`、`~/Documents`、`~/Downloads` 作为演练根目录，但**允许**它们的子目录（例如 `~/Desktop/演练演示`） |
| **规模上限** | 待处理文件数超过 `lock.max_files`（默认 5 万）时中止 |
| **先记录后动手** | 必须先成功写入 `manifest.json` 才会开始改名；写不进去就中止，绝不出现「锁了但没记录」 |
| **只改名不加密** | 全部动作只有一次 `fs::rename`，文件内容零改动 |
| **不跟随符号链接** | 避免顺着软链把范围扩大到目录树之外 |
| **幂等** | 重复运行不会重复加后缀，已锁定的文件会被跳过 |
| **预览模式** | `--dry-run` 可以在动手前看到完整的影响清单 |

---

## 配置项说明

配置文件是 `config/drill.toml`，配置在**编译期**嵌入二进制——所以改完必须重新编译。

### `[organization]` —— 单位信息（必改）

| 字段 | 说明 |
|---|---|
| `name` | 单位全称，会出现在弹窗正文、壁纸、下载页 |
| `short_name` | 单位简称，用于空间较小的位置（如首页图标） |
| `drill_code` | 演练编号，用于演练记录归档 |

### `[lock]` —— 锁定行为

| 字段 | 默认值 | 说明 |
|---|---|---|
| `extension` | `.drill_locked` | 伪加密后缀 |
| `recursive` | `true` | 是否递归子目录 |
| `max_files` | `50000` | 规模上限，超过即中止 |
| `exclude` | `[".git", "target", ...]` | 遍历时跳过的目录/文件名 |

### `[wallpaper]` —— 壁纸

| 字段 | 说明 |
|---|---|
| `width` / `height` | 壁纸分辨率，建议不低于演练机器分辨率 |
| `background` | 背景色，默认纯黑 `#000000` |
| `title` | 主标题（大号红字） |
| `subtitle` | 副标题，支持 `{org}` 占位符与 `\n` 换行 |
| `accent` / `subtitle_color` | 文字颜色 |
| `font_path` | 字体文件路径，留空则自动探测系统中文字体 |
| `drill_notice` | **合规选项**：填入文字会在壁纸右下角显示小字标识（如"应急演练"）；留空则纯拟真 |

### `[popup]` —— 勒索弹窗

| 字段 | 说明 |
|---|---|
| `title` | 窗口标题栏文字 |
| `headline` | 窗口内大标题 |
| `email` / `bitcoin` / `amount` | 虚构的联系邮箱、收款地址与赎金金额 |
| `pay_raise_hours` / `files_lost_hours` | 两个倒计时的时限（小时） |
| `body` | 正文，支持 `{org}` `{amount}` `{email}` 等占位符，空行分段 |
| `show_drill_disclaimer` | 是否在正文末尾追加演练声明（**建议保持开启**） |
| `disclaimer` | 演练声明的文案 |

### `[web]` —— 钓鱼页面

| 字段 | 说明 |
|---|---|
| `site_title` | 仿冒的浏览器主页标题 |
| `download_name` | 快捷方式里那个"工具"的名字 |
| `download_version` / `download_size` / `download_count` | 下载页上的装饰信息 |

---

## 跨平台注意事项

### 打包出的 dist/ 是五平台通用的

`cargo run -p xtask -- package` 会尽可能多地构建各个平台，产物用平台后缀区分：

| 文件 | 平台 |
|---|---|
| `drill-locker-darwin-arm64` | macOS（Apple Silicon） |
| `drill-locker-windows-amd64.exe` | Windows x86_64 |
| `drill-locker-windows-arm64.exe` | Windows on ARM |
| `drill-locker-linux-amd64` | Linux x86_64 |
| `drill-locker-linux-arm64` | Linux arm64 |

`drill-restorer-*` 同理。启动脚本 `start-drill.command` / `.bat` / `.sh`
（以及对应的 `restore.*`）会自动判断机器架构，挑选合适的那份二进制。

整个 `dist/` 目录拷到哪台机器都能直接用，下载页也会按访问者的系统自动指向对应文件。

### 交叉编译工具链

在 macOS 上构建其它平台需要额外装工具链。**缺哪个就跳过哪个**，不影响本机与其他平台打包，
只是对应平台的机器上会没有可用的程序。

```bash
# Windows x86_64
brew install mingw-w64

# Linux amd64 / arm64
brew tap messense/macos-cross-toolchains
brew install x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu

# Windows arm64（llvm-mingw，从 GitHub 下载后解压即可）
# https://github.com/mstorsjo/llvm-mingw/releases
export LLVM_MINGW_BIN=/path/to/llvm-mingw-<版本>-ucrt-macos-universal
rustup target add aarch64-pc-windows-gnullvm
```

`LLVM_MINGW_BIN` 只是让 `xtask` 找到 arm64 的链接器，不设置就跳过该平台。

> 注意：Windows 目标必须用 gnu/gnullvm 变体，MSVC 目标无法在 macOS 上交叉编译。

### macOS

- **首次换壁纸需要授权**：系统会弹出「允许控制『系统事件』吗？」，必须点「好」。
  若误点拒绝，到『系统设置 → 隐私与安全性 → 自动化』中重新勾选。
- 双击 `.command` 文件时若提示无法打开，右键选择「打开」即可。
- 若系统阻止运行未签名的程序，到『系统设置 → 隐私与安全性』中点「仍要打开」。

### Windows

- 安全软件（Defender 等）可能拦截 `drill-locker.exe`，请在演练机器上**临时放行**。
  > 这是安全软件的正确行为——本程序的锁定动作确实与勒索软件相似。
- 壁纸设置通过 `SystemParametersInfoW` 完成，无需额外依赖。
- 建议使用 Windows 10 1803 及以上版本（支持 PNG 壁纸）。

### 字体

壁纸渲染需要在系统中找到中文字体，程序会自动探测：

- **macOS**：冬青黑体 → 华文黑体 → 宋体
- **Windows**：微软雅黑 → 黑体 → 宋体

找不到时会在控制台给出提示，此时可在 `config/drill.toml` 中设置 `wallpaper.font_path` 手动指定。

---

## 常见问题

**Q：锁定了文件之后，把 `manifest.json` 删了还能恢复吗？**

可以，但只能恢复文件名。恢复工具会退化为「去掉所有带该后缀的文件名」的方式，
壁纸则需要手动设置。所以**请勿删除 `manifest.json`**。

**Q：程序拒绝执行，提示"位于系统目录之下"？**

这是安全护栏在起作用。把 `dist/` 移到一个专用文件夹（例如 `~/Desktop/演练演示/`）再运行。

**Q：重复运行 `drill-locker` 会怎样？**

不会重复加后缀。已经带后缀的文件会被跳过，程序是幂等的。

**Q：关闭了勒索弹窗，文件会自己恢复吗？**

不会。这正是演练要传达的点——关掉窗口解决不了问题，必须走恢复流程。
请运行 `drill-restorer` 还原。

**Q：想在壁纸上标注"应急演练"以免参演人员误解？**

在 `config/drill.toml` 中把 `wallpaper.drill_notice` 设为 `"应急演练"`，重新编译即可。
弹窗正文末尾默认也会附上一句演练声明（由 `popup.show_drill_disclaimer` 控制）。

**Q：Windows 上点「立即下载」没反应，或者下载页报 404？**

说明 `dist/` 里没有 `drill-locker.exe`。下载页会按访问者的系统选择文件，
如果打包时跳过了 Windows 版本（缺少 mingw-w64），Windows 用户就会拿到 404。

解决办法：在打包机上装好 mingw-w64 后重新打包。

```bash
brew install mingw-w64            # macOS
cargo run -p xtask -- package
```

打包完成后确认 `dist/` 里同时存在 `drill-locker` 和 `drill-locker.exe`。

---

## 开发

```bash
cargo test                        # 跑单元测试
cargo run -p xtask -- help        # 查看打包工具用法
cargo run -p xtask -- wallpaper   # 只生成壁纸，预览效果
```
