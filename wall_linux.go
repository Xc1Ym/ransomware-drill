//go:build linux

// Ubuntu/Linux 平台实现:GNOME 通过 gsettings 设置壁纸(无桌面环境时静默降级)
package main

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
)

func exeSuffix() string { return "" }

// wallBackup.Raw 约定:"linux|<原 picture-uri>"(无法读取时 Valid=false)

func backupWallpaper() (wallBackup, error) {
	uri, err := gsettings("get", "org.gnome.desktop.background", "picture-uri")
	if err != nil || strings.TrimSpace(uri) == "" {
		// 非 GNOME/无 gsettings:降级为"无备份"
		return wallBackup{Valid: false}, nil
	}
	uri = strings.Trim(strings.TrimSpace(uri), "'")
	if uri == "" {
		return wallBackup{Valid: false}, nil
	}
	return wallBackup{Valid: true, Raw: "linux|" + uri}, nil
}

func applyBlackWallpaper() error {
	img := os.TempDir() + "/wneec_drill_black.png"
	if err := writeBlackPNG(img); err != nil {
		return err
	}
	if _, err := gsettings("set", "org.gnome.desktop.background", "picture-uri", "file://"+img); err != nil {
		return fmt.Errorf("无 GNOME/gsettings(跳过壁纸): %w", err)
	}
	return nil
}

func restoreWallpaper(bk wallBackup) error {
	uri, ok := strings.CutPrefix(bk.Raw, "linux|")
	if !ok || uri == "" {
		return fmt.Errorf("非法壁纸备份记录: %q", bk.Raw)
	}
	_, err := gsettings("set", "org.gnome.desktop.background", "picture-uri", uri)
	return err
}

func showAlert(title, body string) {
	if _, err := zenity("--warning", "--title", title, "--text", body); err != nil {
		// 无 zenity 图形环境:退化为控制台输出
		fmt.Println("[弹窗不可用(无 zenity)],正文见控制台")
	}
}

func platformNotify(msg string) {
	// 有 zenity 时提示,无则静默(控制台已有输出)
	_, _ = zenity("--info", "--timeout=3", "--title=演练提示", "--text="+msg)
}

func gsettings(args ...string) (string, error) {
	cmd := exec.Command("gsettings", args...)
	return strings.TrimSpace(string(mustOutput(cmd))), cmd.Run()
}

func zenity(args ...string) (string, error) {
	cmd := exec.Command("zenity", args...)
	return strings.TrimSpace(string(mustOutput(cmd))), cmd.Run()
}

func mustOutput(cmd *exec.Cmd) []byte {
	out, _ := cmd.Output()
	return out
}
