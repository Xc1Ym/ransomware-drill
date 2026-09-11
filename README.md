# 勒索病毒应急演练演示套件

用于网络安全应急演练的**假**勒索软件。只改文件名后缀，不加密、不删除、不联网，可一键完整还原。

支持 macOS（Apple Silicon）、Windows（x64 / ARM64）、Linux（x64 / ARM64）。

---

## 快速开始

```bash
# 打包（编译 + 渲染页面 + 生成壁纸，产物在 dist/）
cargo run -p xtask -- package

# 把整个 dist/ 拷到演练机器上，然后：

# 先预览会影响哪些文件（不做任何改动）
./dist/drill-locker-darwin-arm64 --dry-run

# 触发演练
./dist/start-drill.command          # macOS 双击
# 或 dist\start-drill.bat           # Windows 双击
# 或 ./dist/start-drill.sh          # Linux

# 恢复
./dist/restore.command
# 或 ./dist/drill-restorer-darwin-arm64 --yes
```

## 换单位演练

只需改 `config/drill.toml`，重新 `cargo run -p xtask -- package`。
单位名会同步到弹窗、壁纸、勒索信、所有页面，以及恢复工具的输出。

## 演练场景

打开 `dist/index.html`，两条最常见的中招路径可选：

| 场景 | 链路 |
|---|---|
| **文件下载勒索** | 仿浏览器新标签页 → 搜「chrome 下载」→ 点置顶「广告」结果 → 仿下载站 → 下载 |
| **邮件勒索** | 仿邮箱收件箱 → 点开「人事部」通知邮件 → 点附件下载 |

下载页会按访问者系统自动选择对应平台的程序。

演练结束后，回到 `index.html` 底部点「下载恢复工具」，即可拿到对应平台的还原工具
（也可直接用 `dist/` 里的 `drill-restorer-*`）。

## 配置

`config/drill.toml` 里几乎所有会随单位、场景变化的东西都能改：

| 配置段 | 管什么 |
|---|---|
| `[organization]` | 单位名、简称、演练编号 |
| `[identity]` | **域名**。内部邮箱后缀、示例网址都由它派生，改一处全站跟着变 |
| `[lock]` | 伪加密后缀、是否递归、规模上限、排除项 |
| `[wallpaper]` | 壁纸分辨率、标语、配色、可选的「演练」水印 |
| `[popup]` | 勒索弹窗的标题、正文、邮箱、比特币地址、赎金、倒计时 |
| `[ransom_note]` | 勒索信文件名、识别码前缀、正文 |
| `[portal]` | 入口页标题与两张场景卡片的文案 |
| `[browser]` | 仿浏览器的标签页标题、地址栏、搜索框提示、快捷方式磁贴 |
| `[search]` | 仿搜索结果页的品牌、栏目、装饰结果、相关搜索 |
| `[download]` → `[web]` | 仿下载站的品牌、导航、软件介绍、功能列表、安装说明、伪装信息 |
| `[mail]` | 邮箱品牌、导航、钓鱼邮件的主题/发件人/正文/附件名、收件箱其它邮件 |

文案里可用 `{org}` `{domain}` `{external_domain}` `{download_name}` 等占位符。

## 安全护栏

- **只重命名文件**，不读改内容、不删除、不加密、不联网
- 系统目录黑名单：`/System`、`C:\Windows`、`/usr`、家目录本身等，命中即中止
- 规模上限：待处理文件超过 `lock.max_files` 时中止
- 先写 manifest 再动手，写不进去就中止；重复运行增量合并，不会丢还原记录
- `--dry-run` 预览；唯一的删除操作是清理演练自己投放的勒索信

## 开发

```bash
cargo test                         # 单元测试
cargo run -p xtask -- help         # 打包工具命令
cargo run -p xtask -- sandbox      # 造一批测试样本
cargo run -p xtask -- wallpaper    # 只生成壁纸，预览效果
```

### 交叉编译工具链

在 macOS 上构建其它平台需要额外装工具链。**缺哪个就跳过哪个**，不影响本机打包。

```bash
brew install mingw-w64                                          # Windows x64
brew tap messense/macos-cross-toolchains                        # Linux x64 / arm64
brew install x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu

# Windows ARM64：从 GitHub 下载 llvm-mingw 解压后指定路径
export LLVM_MINGW_BIN=/path/to/llvm-mingw-<版本>-ucrt-macos-universal
rustup target add aarch64-pc-windows-gnullvm
```

### 注意事项

- 改完 `config/drill.toml` 或 `web/*.html` 后必须重新 `package` 才会生效（配置是编译期嵌入的）
- macOS 首次换壁纸会请求「控制系统事件」权限，需要点「好」
- Windows / Linux 二进制是交叉编译产物，建议在实机上先跑一次 `--dry-run` 确认
- Windows 上安全软件可能拦截，请在演练机器上临时放行
