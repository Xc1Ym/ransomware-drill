package main

// 生成 1x1 纯黑 PNG(供 macOS/Linux 设置壁纸;Windows 使用 BMP,见 wall_windows.go)

import (
	"image"
	"image/color"
	"image/png"
	"os"
)

func writeBlackPNG(path string) error {
	img := image.NewRGBA(image.Rect(0, 0, 320, 180))
	black := color.RGBA{A: 0xff}
	for y := range 180 {
		for x := range 320 {
			img.SetRGBA(x, y, black)
		}
	}
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	defer f.Close()
	return png.Encode(f, img)
}
