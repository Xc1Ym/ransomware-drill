//go:build windows

// Windows 平台实现:注册表壁纸 + Win32 弹窗(原 syscall LazyDLL,零第三方依赖)
package main

import (
	"encoding/binary"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"unsafe"
)

// ---- Win32 API(LazyDLL) ----
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

func exeSuffix() string { return ".exe" }

// wallBackup.Raw 约定:"win|wallpaper|WallpaperStyle|TileWallpaper"(缺失段=原值不存在,还原时删除)

func backupWallpaper() (wallBackup, error) {
	hk, err := regOpen(keyAllAccess)
	if err != nil {
		return wallBackup{}, err
	}
	defer regClose(hk)
	w, wSet, err := regReadString(hk, "Wallpaper")
	if err != nil {
		return wallBackup{}, err
	}
	st, stSet, err := regReadString(hk, "WallpaperStyle")
	if err != nil {
		return wallBackup{}, err
	}
	tl, tlSet, err := regReadString(hk, "TileWallpaper")
	if err != nil {
		return wallBackup{}, err
	}
	return wallBackup{
		Valid: wSet || stSet || tlSet,
		Raw:   "win|" + w + "|" + st + "|" + tl,
		// 无效标记:用 _set 位?为兼容 json,Valid 置 false 时 Raw 依然记录
	}, nil
}

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

func restoreWallpaper(bk wallBackup) error {
	segs := strings.SplitN(bk.Raw, "|", 4)
	if len(segs) < 4 {
		return fmt.Errorf("unknown windows wallpaper backup layout: %q", bk.Raw)
	}
	hk, err := regOpen(keyAllAccess)
	if err != nil {
		return err
	}
	defer regClose(hk)

	if _, _, set, err := winRegValueExistsAndWrite(hk, "Wallpaper", segs[1]); err != nil {
		return err
	} else if !set {
		_ = regDeleteValue(hk, "Wallpaper")
	}
	if _, _, set, err := winRegValueExistsAndWrite(hk, "WallpaperStyle", segs[2]); err != nil {
		return err
	} else if !set {
		_ = regDeleteValue(hk, "WallpaperStyle")
	}
	if _, _, set, err := winRegValueExistsAndWrite(hk, "TileWallpaper", segs[3]); err != nil {
		return err
	} else if !set {
		_ = regDeleteValue(hk, "TileWallpaper")
	}
	setWallpaper(segs[1])
	return nil
}

// winRegValueExistsAndWrite 若段值非空则写入并返回 set=true
func winRegValueExistsAndWrite(hk uintptr, name, val string) (string, bool, bool, error) {
	if val == "" {
		return val, false, false, nil
	}
	if err := regWriteString(hk, name, val); err != nil {
		return val, true, true, err
	}
	return val, true, true, nil
}

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

// showAlert 弹出"你已被勒索!"警告窗(UTF-16)。
func showAlert(title, body string) {
	t, _ := syscall.UTF16PtrFromString(title)
	m, _ := syscall.UTF16PtrFromString(body)
	procMessageBoxW.Call(0, uintptr(unsafe.Pointer(m)), uintptr(unsafe.Pointer(t)),
		uintptr(mbOk|mbIconError))
}

// isGUI 判断是否处于无控制台的 GUI 模式(-H=windowsgui 构建或双击启动时
// GetConsoleWindow 返回 NULL)。
func isGUI() bool {
	r, _, _ := syscall.NewLazyDLL("kernel32.dll").NewProc("GetConsoleWindow").Call()
	return r == 0
}

var guiMode = isGUI()

// platformNotify:CUI 走控制台;GUI 模式补一个 MessageBox,确保用户能看到结果。
func platformNotify(msg string) {
	if guiMode {
		title, _ := syscall.UTF16PtrFromString("演练提示")
		text, _ := syscall.UTF16PtrFromString(msg)
		procMessageBoxW.Call(0, uintptr(unsafe.Pointer(text)), uintptr(unsafe.Pointer(title)),
			uintptr(mbOk|mbIconInformation))
	}
}
