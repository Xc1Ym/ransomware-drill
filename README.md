# 勒索病毒应急演练样本(假)

一个**可逆的**仿真勒索病毒样本，
模拟"机器中勒索病毒"事件,驱动应急响应流程(发现 → 研判 → 处置 → 恢复)。

样本**只改文件名后缀,不加密内容**;无网络外联、无持久化,双击还原器即可一键复原现场。

## 产物(三个 exe)

| 文件 | 用途 |
|------|------|
| `ransom_drill.exe` | **GUI 勒索样本**,双击运行即模拟勒索(无控制台,全流程弹窗) |
| `restore_drill.exe` | **GUI 还原器**,双击即一键还原现场,无需命令行参数 |
| `ransom_drill_console.exe` | CUI 教学版,控制台逐步打印行为日志,供演示者讲解用 |

`ransom_drill.exe` 与 `restore_drill.exe` 为**同一二进制**,运行时按**文件名**自动区分:
`restore_drill.exe` 双击即还原,`ransom_drill.exe` 双击即模拟。

## 样本行为

| 行为 | 说明 |
|------|------|
| 文件锁定 | exe 所在目录(含子目录)所有文件追加 `.wncry` 后缀,**只改名,不改内容** |
| 勒索信 | 每个目录写一份 `如何恢复你的文件.txt`(中文勒索信,含演练标识与加密识别码 WNCRY-2026-xxx) |
| 桌面背景 | 改为纯黑色(首次运行自动备份原壁纸配置,还原时恢复) |
| 弹窗 | MessageBox 弹出「你已被勒索!」中文警告(加密识别码同步展示,便于复盘对位) |

**安全边界**:无加密、无网络外联、无持久化(不写启动项)、不触碰系统目录(`Windows`/`Program Files`)、
重复运行不丢还原记录(日志增量合并)、工作目录固定为 exe 所在目录(双击/命令行行为一致)。

## 构建(Mac 本体)

```bash
./build.sh        # gofmt 检查 → go vet → 交叉编译三个产物(含 -trimpath,不泄漏本机路径)
```

## 部署到 Windows 11 虚拟机

1. 启动 Win11 虚拟机,拷贝 `ransom_drill.exe` / `restore_drill.exe`(教学观察可加
   `ransom_drill_console.exe`)进演练目录,如 `D:\drill`
2. 在演练目录先放几个普通文件:`报告.docx`、`数据.xlsx`、`照片.jpg`

## 演练步骤

```powershell
# 1) 模拟中勒索:双击 D:\drill\ransom_drill.exe(无参数)
#    观察:弹窗「你已被勒索!」→ 桌面变黑 → 文件变 `报告.docx.wncry` → 目录出现勒索信

# 2) 处置/恢复:双击 D:\drill\restore_drill.exe
#    弹窗提示「已还原 N 个文件」→ 文件名还原、勒索信删除、桌面壁纸还原
```

也可以命令行方式(便于教学演示录制):

```powershell
cd D:\drill
.\ransom_drill.exe -restore   # 还原(与双击 restore_drill.exe 等价)
.\ransom_drill_console.exe    # 教学版:观察每一步控制台输出
```

还原完成后,演练日志归档为 `.wncry_drill_log.restored.json` 留档复盘。

## 钓鱼下载页(模拟"恶意下载通道")

`index.html` 是**仿 Google Chrome 官方下载页**(google.com/chrome 观感):
顶栏 + Hero「快速、安全、简单。」+「下载 Chrome」按钮。点击按钮实际下载的是
`ransom_drill.exe`(页面底部固定黄色演练标识条,防误认真实威胁)。

```bash
# 在内网演练服务器(或任意一台演练机器)上,当前目录直接托管:
python3 -m http.server 8080
```

参演人员浏览器访问 `http://<服务器IP>:8080/` → 看到仿 Chrome 下载页 → 点击
「下载 Chrome」→ 获得 `ransom_drill.exe` → 本地执行(勒索效果触发)。

- 页面为纯离线 HTML(内联 CSS/JS,Chrome 图标 CSS 绘制,无外部资源请求)
- 模拟"恶意下载通道"传播链路:仿官方下载页 → 勒索样本落地
- 建议:借演练专用主机/靶机做服务器,网段限定演练范围

### 复用说明:换单位/换样本只改 `index.html` 顶部 `CONFIG`

```js
const CONFIG = {
  theme: 'chrome',                  // chrome:仿浏览器官方下载页(四色环)| generic:泛用蓝盾
  fileName: 'ransom_drill.exe',     // 点击下载的文件名(与样本名保持一致)
  brandName: 'Google Chrome',       // 顶栏品牌名,可换成其它产品/单位名
  unitName: 'XXX市XXX局',            // 演练单位名(出现在底部演练标识)
  mainTitle: '快速、安全、简单。',    // 大标题
  subTitle: '使用 Chrome 浏览器运行 Google 应用。',
  btnText: '下载 Chrome',           // 按钮文字(顶栏/导航/主按钮同步)
  heroMeta: '适用于 Windows 11 · 系统要求 · 下载大小约 2.1 MB'
};
```

零代码换场景:改 `unitName`(换单位)+ `fileName`(换样本)+ `brandName`/`btnText`(换"品牌",如
`XXX省XXX厅 / 系统更新客户端`)。演练标识条自动拼装单位名与文件名。

## 常见问题

- **还原弹窗提示"未找到演练日志"?** 检查是否把 `restore_drill.exe` 放在了**和第一次运行 exe 不同的目录**;
  日志(`.wncry_drill_log.json`)就留在 exe 目录里,还原器必须和 exe 在同一个目录。
- **重复运行了勒索样本?** 幂等安全:已 `.wncry` 的文件不会重复改名,日志自动增量合并,双击一次
  `restore_drill.exe` 仍可全部还原。
- **还原失败想手动恢复?** 文件名只是追加了 `.wncry` 后缀,删掉后缀即可;壁纸可到
  设置 → 个性化 → 背景 重新选择。

## 注意事项

- 仅限隔离的演练虚拟机内运行,不要在生产/重要机器上执行
- 加密识别码(WNCRY-2026-xxx)为演练内唯一标识,对应演练复盘

