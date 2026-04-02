# Yuralock (由良锁)

技术栈：Tauri + **Rust** + Vue

**Android**, **Windows**, **Linux**跨平台全兼容！

（MacOS预期中，暂无测试环境）

特点：支持部分加密，文件头伪装，加解密速度极快，自带文件名恢复，文件完整性校验，内存，存储占用低

------

技术特征后面说，先赛博斗蛐蛐

测试设备：i5-1155g7

### 时长对比

文件大小为1.23gb

Encrypto加解密时长：2m52s57 / 21s97

Yuralock加解密时长（加密比例100）：2s6 / 2s7

### 内存占用

Encrypto常态/加密时：~100mb / ~120mb

Yuralock常态/加密时：～30mb / ~30mb

### 存储占用

Encrypto：4.61mb

Yuralock：linux为8.7mb, windows安装包4.0mb,不包括webview仅6.0mb

## 特性详述

支持零加密至全加密，可以自定义文件加密部分占比

识别码信息使用特殊密钥进行加密，文件任意信息不依赖文件名或后缀

加密比例为0时不加密文件，但是设置伪装层，记录文件头信息（文件头会被加密），写入sha256校验码

全加密时将全部文件加密，设置伪装层，记录文件头信息（被加密），写入sha256校验码

部分加密时将文件前x%加密，设置伪装层，记录文件头信息（被加密），写入sha256校验码，x可自定义


## TODO

❌ 预计支持AES256, ChaCha20, ML-KEM

❌ 支持查看加/解密进度，当前加解密时程序会静默直到加/解密完成

❌ io_uring

✅ Android兼容

❌ MacOs兼容


## 代码占比

yuralock库：全古法编程

yuralock_tauri后端：70%古法编程+review

yuralock_tauri前端：30%古法编程+review