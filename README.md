<img width="1024" height="400" alt="title" src="https://github.com/user-attachments/assets/599ba073-a81a-45da-b3f4-ef480206cd6f" />

<div align="center">
<a href="https://github.com/Xiaxiaobaii/Yuralock/blob/main/README_zh.md">简体中文</a>


YuraLock is a semi-open-source, highly censorship-resistant fast symmetric encryption tool. It supports cross-platform use on Windows, Android, and Linux. It integrates various advanced encryption features while maintaining extremely low resource usage, and is built with the Tauri framework for a lightweight and efficient experience.
</div>

<img width="1165" height="803" alt="example1" src="https://github.com/user-attachments/assets/41321899-6779-49b0-9d08-7238ae07ddec" />  

## Key Features

1. Approximately 90x faster than Encrypto.

2. Supports cross-platform Windows, Android and Linux.

3. Almost no identifiable features.

4. Auto save filename.

5. Built-in file integrity verification

## Quick Start

1. Go Release pages to download latest file.

2. Open the App, tap "Select File", and choose the file you want to encrypt.

3. In the "Enter Key" input box, enter the key you want to use.

4. Slide the "Encryption Ratio" slider to set the encryption ratio for the file. For text files, important files, or streaming files, it is recommended to use 100% encryption. This will not cause significant time consumption.

5. Tap "Start Encryption". When the progress bar is complete, a pop-up window will appear telling you where the encrypted file has been saved. It is usually a .zip file with a random string of English characters as the filename, stored in the same directory as the original file.

6. After waiting for a while (or sharing the encrypted file with others, or doing nothing), open the App, select the encrypted file, enter the key you set in step 3, and tap "Start Decryption". Once decryption is complete, a pop-up window will appear telling you where the decrypted file is stored (it is generally saved in the same location as the encrypted file, with the same filename as the original file you selected during encryption).

## Benchmark comparison with Encrypto

Test Device cpu is I5-1155g7

platform: Windows11 and x86_64

File size: 1.23GB

### Encrypt/Decrypt Time

Yuralock：2s6 / 2s7

Encrypto：2m52s57 / 21s97

### RAM Use

Encrypto：~100mb-120mb

Yuralock：~30mb

### App Size

Encrypto：4.61mb

Yuralock：linux[8.56 MB] windows[3.86 MB] android[34.4 MB]

## TODO

❌ Support AES256, ChaCha20, ML-KEM

✅ Look Encrypt/Decrypt process.

❌ io_uring

✅ Android Support

❌ MacOs Support
