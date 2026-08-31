// 假勒索病毒演练样本(应急演练专用)
//
// 仅用于XXX市XXX局应急演练:
//   - 不加密文件内容,只追加 .wncry 后缀(可逆)
//   - 无网络外联、无持久化(不写启动项)
//   - 每个目录落一份中文勒索信;弹窗/壁纸动作由平台插件实现(wall_*.go),无效时静默降级
//   - 运行 ransomware_drill -restore 一键还原
//
// 构建(多平台交叉编译):
//
//	./build.sh
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

const (
	ransomSuffix  = ".wncry"                         // 勒索后缀
	ransomNote    = "如何恢复你的文件.txt"                   // 勒索信文件名
	drillLogName  = ".wncry_drill_log.json"          // 演练日志(改名映射/壁纸备份)
	drillLogDone  = ".wncry_drill_log.restored.json" // 恢复完成后的归档日志
	drillEmail    = "security-drill@example.com"
	drillIDPrefix = "WNCRY-2026-"
)

const ransomText = `!!! 你已被勒索 !!!
您的全部重要文件已被加密并锁定,所有文件现均变为 %s 格式,无法正常打开。

【您的文件】
当前目录及其所有子目录中的文件均已被锁定。

【如何解除】
按以下步骤处理,您的文件才会被解密:
1. 支付赎金(演练环境:请勿支付任何费用)
2. 只接受 72 小时内的支付,超时赎金翻倍
3. 严禁自行重装系统或使用暴力破解,否则文件将永久丢失

联系方式: %s
加密识别码: %s

【应急演练·模拟样本】
本样本由XXX市XXX局应急演练专用:
未加密文件内容、未损坏任何数据、无网络连接行为。
演练结束后运行 ransomware_drill -restore 可一键还原所有文件名与桌面设置。
`

// ---- 演练日志结构(恢复模式的依据) ----
type fileEntry struct {
	Old string `json:"old"` // 原名(绝对路径)
	New string `json:"new"` // 勒索名(绝对路径)
}

// wallBackup 由各平台插件编码/解码(wall_backup 内容见 *_wall.go 的注释约定)
type wallBackup struct {
	Raw   string `json:"wallpaper"`
	Valid bool   `json:"wallpaper_valid"`
}

type drillLog struct {
	CreatedAt       string      `json:"created_at"`
	WallpaperBacked bool        `json:"wallpaper_backed"` // 原壁纸配置是否已备份(重复运行只备一次)
	RansomNotes     []string    `json:"ransom_notes"`
	Wallpaper       wallBackup  `json:"wallpaper_backup"`
	Files           []fileEntry `json:"files"`
}

// ---- 平台插件接口(每个平台一个实现文件) ----
// wall_windows.go / wall_darwin.go / wall_linux.go 必须提供:
//
//	backupWallpaper() (wallBackup, error)       备份原壁纸(失败返回 Valid=false)
//	applyBlackWallpaper() error                 设置纯黑壁纸
//	restoreWallpaper(bk wallBackup) error       还原壁纸
//	showAlert(title, body string)               弹"你已被勒索"通告(无图形环境时静默降级为打印)
//	platformNotify(msg string)                  "完成/错误"提示:控制台之外的通知(无则空实现)
//	exeSuffix() string                          ".exe"(Windows)或 ""

func main() {
	exePath := mustExecutable()
	exeDir := filepath.Dir(exePath)

	// 固定工作目录为 exe 所在目录:保证「双击运行」与「命令行运行」行为一致。
	// Windows 资源管理器双击启动时工作目录不保证是 exe 目录,不固定会导致
	// 文件改错目录、日志找不到、无法还原。
	if err := os.Chdir(exeDir); err != nil {
		notice("[错误] 无法切换到程序目录:" + err.Error())
		os.Exit(1)
	}

	// 无参数 = 模拟勒索;带 -restore,或以 restore 开头的文件名运行(如
	// restore_drill,双击即还原,无需命令行) = 还原现场。
	restore := flag.Bool("restore", false, "一键还原被改名的文件与桌面设置")
	flag.Parse()
	logPath := filepath.Join(".", drillLogName)
	if *restore || strings.HasPrefix(strings.ToLower(filepath.Base(os.Args[0])), "restore") {
		if err := restoreAll(logPath); err != nil {
			notice("[错误] 还原失败:" + err.Error())
			os.Exit(1)
		}
		return
	}

	// ---- 1. 收集当前目录(递归)下所有普通文件 ----
	exeName := filepath.Base(exePath)
	files := collectFiles(".", exeName)
	if len(files) == 0 {
		notice("[提醒] 当前目录下没有可处理的文件")
	} else {
		fmt.Printf("[行为 1/4] 正在锁定 %d 个文件...\n", len(files))
	}

	// ---- 2. 递归追加 .wncry 后缀(只改名,内容不动) ----
	// 重复运行读取已有日志并增量合并,避免覆盖映射导致 -restore 无法还原。
	log := &drillLog{CreatedAt: time.Now().Format("2006-01-02 15:04:05")}
	if old, err := readJSON(logPath); err == nil {
		log = old
		fmt.Println("[提示] 检测到已有演练记录,本次增量合并")
	}
	lockedNew := map[string]bool{}
	for _, e := range log.Files {
		lockedNew[e.New] = true
	}
	added := 0
	for _, p := range files {
		if strings.HasSuffix(strings.ToLower(p), ransomSuffix) {
			continue // 已处理过,幂等保护
		}
		newName := p + ransomSuffix
		if _, err := os.Stat(newName); err == nil {
			continue // 目标名已存在,跳过避免覆盖
		}
		if err := os.Rename(p, newName); err != nil {
			fmt.Printf("[跳过] %s : %v\n", p, err)
			continue
		}
		if !lockedNew[abs(newName)] {
			log.Files = append(log.Files, fileEntry{Old: abs(p), New: abs(newName)})
			lockedNew[abs(newName)] = true
		}
		added++
		fmt.Println("[锁定] " + p + " -> " + newName)
	}

	// ---- 3. 当前目录及子目录落勒索信 ----
	fmt.Println("[行为 2/4] 正在写入勒索信...")
	notesDirs := map[string]bool{".": true}
	for _, d := range topDirs(files) {
		notesDirs[d] = true
	}
	ransomID := drillIDPrefix + time.Now().Format("01-02-150405")
	noteBody := fmt.Sprintf(ransomText, ransomSuffix, drillsEmail(), ransomID)
	lockedNotes := map[string]bool{}
	for _, n := range log.RansomNotes {
		lockedNotes[n] = true
	}
	for d := range notesDirs {
		p := filepath.Join(d, ransomNote)
		if err := os.WriteFile(p, []byte(noteBody), 0644); err != nil {
			fmt.Printf("[跳过] 写勒索信 %s : %v\n", p, err)
			continue
		}
		if !lockedNotes[abs(p)] {
			log.RansomNotes = append(log.RansomNotes, abs(p))
		}
		fmt.Println("[勒索信] " + p)
	}

	// ---- 4. 桌面背景改纯黑(首次运行备份原壁纸配置) ----
	fmt.Println("[行为 3/4] 正在篡改桌面壁纸...")
	if !log.WallpaperBacked {
		bk, err := backupWallpaper()
		if err != nil {
			fmt.Println("[警告] 备份壁纸失败(继续):", err)
		} else {
			log.Wallpaper = bk
			log.WallpaperBacked = true
		}
	}
	if err := applyBlackWallpaper(); err != nil {
		fmt.Println("[警告] 设置黑色壁纸失败:", err)
	}

	// ---- 5. 弹窗:你已被勒索 ----
	fmt.Println("[行为 4/4] 弹出勒索通告...")
	showAlert("你已被勒索! "+ransomID, noteBody)

	// ---- 6. 写演练日志(恢复依据) ----
	if err := writeJSON(logPath, log); err != nil {
		fmt.Println("[警告] 写日志失败,恢复模式将不可用:", err)
	}
	notice(fmt.Sprintf("[完成] 本次新增锁定 %d 个文件,累计 %d 个。还原方法:运行 restore_drill%s。",
		added, len(log.Files), exeSuffix()))
}

// collectFiles 递归收集当前目录下可处理的普通文件。
// 跳过:自身 exe、日志/勒索信文件、系统保护目录(Windows、Program Files)。
func collectFiles(root, exeName string) []string {
	var files []string
	_ = filepath.Walk(root, func(p string, info os.FileInfo, err error) error {
		if err != nil {
			return nil // 无权限/已删除的条目容忍跳过
		}
		if info.IsDir() {
			name := strings.ToLower(info.Name())
			if p != "." && (name == "windows" || strings.HasPrefix(name, "program files")) {
				return filepath.SkipDir // 防误伤系统
			}
			return nil
		}
		base := filepath.Base(p)
		if strings.EqualFold(base, exeName) ||
			strings.EqualFold(base, drillLogName) ||
			strings.EqualFold(base, drillLogDone) ||
			strings.EqualFold(base, ransomNote) {
			return nil
		}
		files = append(files, p)
		return nil
	})
	return files
}

// topDirs 返回文件中涉及的顶层子目录(用于放置勒索信)。
func topDirs(files []string) []string {
	seen := map[string]bool{}
	var dirs []string
	for _, f := range files {
		d := filepath.Dir(f)
		if d == "." || seen[d] {
			continue
		}
		seen[d] = true
		dirs = append(dirs, d)
	}
	return dirs
}

// ---- 恢复模式 ----

func restoreAll(logPath string) error {
	log, err := readJSON(logPath)
	if err != nil {
		return fmt.Errorf("未找到演练日志 %s(可能已还原或不在演练目录运行): %w", logPath, err)
	}
	fmt.Println("[还原 1/4] 恢复文件名...")
	restored := 0
	for _, e := range log.Files {
		if _, err := os.Stat(e.Old); err == nil {
			continue // 原名已存在(用户改回去了),跳过
		}
		if err := os.Rename(e.New, e.Old); err != nil {
			fmt.Println("[跳过] 还原 " + e.New + ":" + err.Error())
			continue
		}
		restored++
	}

	fmt.Println("[还原 2/4] 删除勒索信...")
	for _, p := range log.RansomNotes {
		if err := os.Remove(p); err != nil {
			fmt.Println("[跳过] " + p + ":" + err.Error())
		}
	}

	fmt.Println("[还原 3/4] 恢复桌面壁纸...")
	if log.Wallpaper.Valid {
		if err := restoreWallpaper(log.Wallpaper); err != nil {
			fmt.Println("[警告] 恢复壁纸失败(继续):", err)
		}
	}

	fmt.Println("[还原 4/4] 归档演练日志...")
	if err := os.Rename(logPath, filepath.Join(filepath.Dir(logPath), drillLogDone)); err != nil {
		fmt.Println("[警告] 归档日志失败:", err)
	}

	notice(fmt.Sprintf("[完成] 已还原 %d 个文件。演练现场已复原。", restored))
	return nil
}

// ---- 演练提示输出 ----

// notice 输出演练提示:始终打印控制台;平台补充系统通知(见 platformNotify)。
func notice(msg string) {
	fmt.Println(msg)
	platformNotify(msg)
}

// ---- 小工具 ----

func drillsEmail() string { return drillEmail }

func abs(p string) string {
	if a, err := filepath.Abs(p); err == nil {
		return a
	}
	return p
}

func mustExecutable() string {
	exe, err := os.Executable()
	if err != nil {
		fmt.Println("[错误] 无法获取可执行文件路径:", err)
		os.Exit(1)
	}
	return exe
}

func writeJSON(path string, v any) error {
	data, err := json.MarshalIndent(v, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, data, 0644)
}

func readJSON(path string) (*drillLog, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var log drillLog
	if err := json.Unmarshal(data, &log); err != nil {
		return nil, err
	}
	return &log, nil
}
