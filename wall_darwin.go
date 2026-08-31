//go:build darwin

// macOS 平台实现:通过 osascript(System Events)设置壁纸/弹窗(借助辅助权限)
package main

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
)

func exeSuffix() string { return "" }

// wallBackup.Raw 约定:"darwin|<原壁纸路径>"(无法读取时 Valid=false)

func backupWallpaper() (wallBackup, error) {
	out, err := osascript(`tell application "System Events" to get picture of every desktop`)
	if err != nil || strings.TrimSpace(out) == "" {
		// 无辅助权限/无 System Events:降级为"无备份",后续 apply 失败也不影响主流程
		return wallBackup{Valid: false}, nil
	}
	first, _, _ := strings.Cut(strings.TrimSpace(out), "\n")
	first = strings.TrimSpace(first)
	if first == "" {
		return wallBackup{Valid: false}, nil
	}
	return wallBackup{Valid: true, Raw: "darwin|" + first}, nil
}

func applyBlackWallpaper() error {
	img := os.TempDir() + "/wneec_drill_black.png"
	if err := writeBlackPNG(img); err != nil {
		return err
	}
	_, err := osascript(fmt.Sprintf(`tell application "System Events" to set picture of every desktop to POSIX file "%s"`, img))
	if err != nil {
		return fmt.Errorf("设置壁纸需 System Events 辅助权限(无则跳过): %w", err)
	}
	return nil
}

func restoreWallpaper(bk wallBackup) error {
	cur, ok := strings.CutPrefix(bk.Raw, "darwin|")
	if !ok || cur == "" {
		return fmt.Errorf("非法壁纸备份记录: %q", bk.Raw)
	}
	_, err := osascript(fmt.Sprintf(`tell application "System Events" to set picture of every desktop to POSIX file "%s"`, cur))
	return err
}

func showAlert(title, body string) {
	// display dialog 需要辅助权限;失败时退回控制台输出
	script := fmt.Sprintf(`display dialog "%s" with title "%s" buttons {"确定"} default button "确定" with icon critical`, appleQuote(body), appleQuote(title))
	out, err := osascript(script)
	if err != nil || strings.TrimSpace(out) == "" {
		fmt.Println("[弹窗不可用(Script/app权限)],正文见控制台")
	}
}

func platformNotify(msg string) {
	// 低打扰:系统通知,失败静默
	_, _ = osascript(fmt.Sprintf(`display notification "%s" with title "演练提示"`, appleQuote(msg)))
}

// osascript 执行并返回 stdout(trim)。
func osascript(script string) (string, error) {
	cmd := exec.Command("osascript", "-e", script)
	out, err := cmd.Output()
	return strings.TrimSpace(string(out)), err
}

// appleQuote 转义 AppleScript 双引号,保证中文/引号安全。
func appleQuote(s string) string {
	return strings.ReplaceAll(s, `"`, `\"`)
}
