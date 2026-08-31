//go:build windows

// 假勒索病毒演练样本(应急演练专用)
//
// 仅用于XXX市XXX局应急演练:
//   - 不加密文件内容,只追加 .wncry 后缀(可逆)
//   - 无网络外联、无持久化(不写启动项)
//   - 每个目录落一份中文勒索信,桌面背景改纯黑,弹窗显示被勒索警告
//
// 构建(在 macOS 上交叉编译):
//
//	CGO_ENABLED=0 GOOS=windows GOARCH=arm64 go build -ldflags "-s -w" -o ransom_drill.exe .
package main

import (
	"encoding/binary"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"time"
	"unsafe"
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
演练结束后运行 ransomware_drill.exe -restore 可一键还原所有文件名与桌面设置。
`

// ---- Win32 API(LazyDLL,零第三方依赖) ----
var (
	user32   = syscall.NewLazyDLL("user32.dll")
	advapi32 = syscall.NewLazyDLL("advapi32.dll")

	procMessageBoxW           = user32.NewProc("MessageBoxW")
	procSystemParametersInfoW = user32.NewProc("SystemParametersInfoW")

	procRegOpenKeyExW    = advapi32.NewProc("RegOpenKeyExW")
	procRegQueryValueExW = advapi32.NewProc("RegQueryValueExW")
	procRegSetValueExW   = advapi32.NewProc("RegSetValueExW")
	procRegDeleteValueW  = advapi32.NewProc("RegDeleteValueW")
	procRegCloseKey      = advapi32.NewProc("RegCloseKey")
)

const (
	spiSetDeskWallpaper = 0x0014
	spifUpdateIniFile   = 0x01
	spifSendChange      = 0x02
	mbOk                = 0x00000000
	mbIconError         = 0x00000010
	mbIconInformation   = 0x00000040

	hkeyCurrentUser = 0x80000001
	keyAllAccess    = 0xF003F
	regSZ           = 1
	errFileNotFound = 2
	desktopRegKey   = `Control Panel\Desktop`
)

// ---- 演练日志结构(恢复模式的依据) ----
type fileEntry struct {
	Old string `json:"old"` // 原名(绝对路径)
	New string `json:"new"` // 勒索名(绝对路径)
}

type wallpaperBackup struct {
	Wallpaper      string `json:"wallpaper"`
	WallpaperSet   bool   `json:"wallpaper_set"`
	WallpaperStyle string `json:"wallpaper_style"`
	StyleSet       bool   `json:"style_set"`
	TileWallpaper  string `json:"tile_wallpaper"`
	TileSet        bool   `json:"tile_set"`
}

type drillLog struct {
	CreatedAt       string          `json:"created_at"`
	WallpaperBacked bool            `json:"wallpaper_backed"` // 原壁纸配置是否已备份(重复运行只备一次)
	RansomNotes     []string        `json:"ransom_notes"`
	Wallpaper       wallpaperBackup `json:"wallpaper_backup"`
	Files           []fileEntry     `json:"files"`
}

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
	// restore_drill.exe,GUI 版双击即还原,无需命令行) = 还原现场。
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
	showMessageBox(ransomID, noteBody)

	// ---- 6. 写演练日志(恢复依据) ----
	if err := writeJSON(logPath, log); err != nil {
		fmt.Println("[警告] 写日志失败,恢复模式将不可用:", err)
	}
	notice(fmt.Sprintf("[完成] 本次新增锁定 %d 个文件,累计 %d 个。还原方法:双击 restore_drill.exe(或运行 ransomware_drill.exe -restore)。",
		added, len(log.Files)))
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
	if log.Wallpaper.WallpaperSet || log.Wallpaper.StyleSet || log.Wallpaper.TileSet {
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

// ---- 注册表(壁纸) ----

func regOpen(access uint32) (uintptr, error) {
	sub, _ := syscall.UTF16PtrFromString(desktopRegKey)
	var hk uintptr
	r, _, err := procRegOpenKeyExW.Call(
		uintptr(hkeyCurrentUser), uintptr(unsafe.Pointer(sub)), 0,
		uintptr(access), uintptr(unsafe.Pointer(&hk)))
	if r != 0 {
		return 0, fmt.Errorf("RegOpenKeyExW: %w", err)
	}
	return hk, nil
}

func regReadString(hk uintptr, name string) (string, bool, error) {
	np, _ := syscall.UTF16PtrFromString(name)
	var typ, size uint32
	r, _, _ := procRegQueryValueExW.Call(hk, uintptr(unsafe.Pointer(np)), 0,
		uintptr(unsafe.Pointer(&typ)), 0, uintptr(unsafe.Pointer(&size)))
	if r != 0 {
		return "", false, nil // 值不存在(2)或其它错误:视为"无备份"
	}
	if size == 0 {
		return "", true, nil
	}
	buf := make([]uint16, size/2+1)
	r, _, err := procRegQueryValueExW.Call(hk, uintptr(unsafe.Pointer(np)), 0,
		uintptr(unsafe.Pointer(&typ)), uintptr(unsafe.Pointer(&buf[0])), uintptr(unsafe.Pointer(&size)))
	if r != 0 {
		return "", false, fmt.Errorf("RegQueryValueExW(%s): %w", name, err)
	}
	return syscall.UTF16ToString(buf), true, nil
}

func regWriteString(hk uintptr, name, val string) error {
	np, _ := syscall.UTF16PtrFromString(name)
	vp, _ := syscall.UTF16PtrFromString(val)
	r, _, err := procRegSetValueExW.Call(hk, uintptr(unsafe.Pointer(np)), 0,
		uintptr(regSZ), uintptr(unsafe.Pointer(vp)), uintptr((len(val)+1)*2))
	if r != 0 {
		return fmt.Errorf("RegSetValueExW(%s): %w", name, err)
	}
	return nil
}

func regDeleteValue(hk uintptr, name string) error {
	np, _ := syscall.UTF16PtrFromString(name)
	r, _, _ := procRegDeleteValueW.Call(hk, uintptr(unsafe.Pointer(np)))
	if r == errFileNotFound {
		return nil
	}
	if r != 0 {
		return syscall.Errno(r)
	}
	return nil
}

func regClose(hk uintptr) {
	procRegCloseKey.Call(hk)
}

// backupWallpaper 读备份 HKCU\Control Panel\Desktop 的三个壁纸键。
func backupWallpaper() (wallpaperBackup, error) {
	hk, err := regOpen(keyAllAccess)
	if err != nil {
		return wallpaperBackup{}, err
	}
	defer regClose(hk)

	var bk wallpaperBackup
	bk.Wallpaper, bk.WallpaperSet, err = regReadString(hk, "Wallpaper")
	if err != nil {
		return bk, err
	}
	bk.WallpaperStyle, bk.StyleSet, err = regReadString(hk, "WallpaperStyle")
	if err != nil {
		return bk, err
	}
	bk.TileWallpaper, bk.TileSet, err = regReadString(hk, "TileWallpaper")
	return bk, err
}

// applyBlackWallpaper 生成纯黑 BMP 写入临时目录,并设为桌面壁纸。
func applyBlackWallpaper() error {
	bmp := filepath.Join(os.Getenv("TEMP"), "drill_wallpaper_black.bmp")
	if err := writeBlackBMP(bmp, 1920, 1080); err != nil {
		return err
	}
	hk, err := regOpen(keyAllAccess)
	if err != nil {
		return err
	}
	defer regClose(hk)
	if err := regWriteString(hk, "Wallpaper", bmp); err != nil {
		return err
	}
	if err := regWriteString(hk, "WallpaperStyle", "10"); err != nil { // 填充
		return err
	}
	if err := regWriteString(hk, "TileWallpaper", "0"); err != nil {
		return err
	}
	setWallpaper(bmp)
	return nil
}

func restoreWallpaper(bk wallpaperBackup) error {
	hk, err := regOpen(keyAllAccess)
	if err != nil {
		return err
	}
	defer regClose(hk)

	if bk.WallpaperSet {
		if err := regWriteString(hk, "Wallpaper", bk.Wallpaper); err != nil {
			return err
		}
	} else {
		if err := regDeleteValue(hk, "Wallpaper"); err != nil {
			return err
		}
	}
	if bk.StyleSet {
		if err := regWriteString(hk, "WallpaperStyle", bk.WallpaperStyle); err != nil {
			return err
		}
	} else {
		if err := regDeleteValue(hk, "WallpaperStyle"); err != nil {
			return err
		}
	}
	if bk.TileSet {
		if err := regWriteString(hk, "TileWallpaper", bk.TileWallpaper); err != nil {
			return err
		}
	} else {
		if err := regDeleteValue(hk, "TileWallpaper"); err != nil {
			return err
		}
	}
	setWallpaper(bk.Wallpaper)
	return nil
}

// setWallpaper 调用 SystemParametersInfoW 即时刷新壁纸。
func setWallpaper(path string) {
	p, _ := syscall.UTF16PtrFromString(path)
	procSystemParametersInfoW.Call(spiSetDeskWallpaper, 0,
		uintptr(unsafe.Pointer(p)), uintptr(spifUpdateIniFile|spifSendChange))
}

// writeBlackBMP 生成 24 位纯黑 BMP(像素区全零)。
func writeBlackBMP(path string, w, h int) error {
	rowSize := (w*3 + 3) &^ 3
	imgSize := rowSize * h
	data := make([]byte, 54+imgSize)
	le := binary.LittleEndian
	copy(data[0:2], "BM")
	le.PutUint32(data[2:6], uint32(len(data))) // 文件大小
	le.PutUint32(data[10:14], 54)              // 像素数据偏移
	le.PutUint32(data[14:18], 40)              // BITMAPINFOHEADER 大小
	le.PutUint32(data[18:22], uint32(w))
	le.PutUint32(data[22:26], uint32(h))
	le.PutUint16(data[26:28], 1) // planes
	le.PutUint16(data[28:30], 24)
	le.PutUint32(data[30:34], 0) // BI_RGB
	le.PutUint32(data[34:38], uint32(imgSize))
	le.PutUint32(data[38:42], 2835)
	le.PutUint32(data[42:46], 2835)
	return os.WriteFile(path, data, 0644)
}

// showMessageBox 弹出"你已被勒索!"警告窗(UTF-16)。
func showMessageBox(ransomID, body string) {
	title, _ := syscall.UTF16PtrFromString("你已被勒索! " + ransomID)
	text, _ := syscall.UTF16PtrFromString(body)
	procMessageBoxW.Call(0, uintptr(unsafe.Pointer(text)), uintptr(unsafe.Pointer(title)),
		uintptr(mbOk|mbIconError))
}

// ---- 演练提示输出 ----

// isGUI 判断是否处于无控制台的 GUI 模式(-H=windowsgui 构建或双击启动时
// GetConsoleWindow 返回 NULL)。
func isGUI() bool {
	r, _, _ := syscall.NewLazyDLL("kernel32.dll").NewProc("GetConsoleWindow").Call()
	return r == 0
}

var guiMode = isGUI()

// notice 输出演练提示:CUI 走控制台,GUI 模式补一个 MessageBox,确保用户能看到结果。
func notice(msg string) {
	fmt.Println(msg)
	if guiMode {
		title, _ := syscall.UTF16PtrFromString("演练提示")
		text, _ := syscall.UTF16PtrFromString(msg)
		procMessageBoxW.Call(0, uintptr(unsafe.Pointer(text)), uintptr(unsafe.Pointer(title)),
			uintptr(mbOk|mbIconInformation))
	}
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
