<img width="1024" height="400" alt="title" src="https://github.com/user-attachments/assets/599ba073-a81a-45da-b3f4-ef480206cd6f" />

<div align="center">
<a href="https://github.com/Xiaxiaobaii/Yuralock/blob/master/README.md">English</a>

由良锁(Yuralock) 是一款半开源、高度反审查的快速对称加密工具，支持 Windows、Android、Linux 三端跨平台使用。它集成多种先进加密技术，同时保持极低的资源占用，基于 Tauri 框架开发，轻量高效。
</div>

<img width="1165" height="803" alt="example1" src="https://github.com/user-attachments/assets/41321899-6779-49b0-9d08-7238ae07ddec" />

## 主要特性

1. 约比Encrypto快9倍！

2. 支持Windows, Linux, Android三端集成！

3. 几乎没有任何可识别的特征, 无外部信息依赖

4. 自动存储加密时的文件名信息，并在解密时还原

5. 自带文件校验

6. 运行时内存占用低至30mb且使用流式加密

--------

## 快速体验

1. 前往Release下载最新版本的Yuralock

2. 打开App, 点击"选择文件", 选择你要加密的文件

3. 在"输入密钥"输入框输入你要使用的密钥

4. 滑动"加密比例"滑动条决定对文件使用的加密比，如果是文本文件或较重要，流式文件，建议使用100%加密，这并不会带来太大的时间消耗

5. 点击"开始加密", 进度条完成时会有弹窗告诉您加密后的文件存储在哪个位置，一般是一个随机英文字符的zip后缀文件并存储在和被加密文件相同的目录

6. 等待一段时间，或者将加密文件分享给别人，或者什么都不做，打开App,选择加密后的文件，输入在第三部设置的密钥，点击"开始解密"，等待解密完成后会有弹窗告诉您解密后的文件存储在哪个位置（一般是存储在与加密文件相同的位置，并且文件名与加密时选择文件的文件名相同）

## 对比测试

测试cpu：i5-1155g7

系统：Windows11, x86_64

文件大小: 1.23gb

### 加解密时间

Yuralock：2s6 / 2s7

Encrypto：2m52s57 / 21s97

### 内存占用

Yuralock：~30mb

Encrypto：~100mb-120mb

### 存储占用

Encrypto：4.61mb

Yuralock：linux[8.56 MB] windows[3.86 MB] android[34.4 MB]
## TODO

❌ 预计支持AES256, ChaCha20, ML-KEM

✅ 支持查看加/解密进度，当前加解密时程序会静默直到加/解密完成

❌ io_uring

✅ Android兼容

❌ MacOs兼容