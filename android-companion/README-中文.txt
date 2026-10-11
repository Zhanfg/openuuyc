AXY Portable Android Companion / v0.1

性质：可独立安装的 Android APK 项目，使用同一 U 盘的数据目录。不依赖 Termux / Root。
目的：从手机或平板访问 Windows 版 AXY-DevStack 的数据，快速打开 Bettbox、KDBX、仓库等。

已实现（源代码层面）：
1. Android 系统文件选择器选择 AXY-DevStack 目录（SAF）；持久保存授权 URI。
2. 从 U 盘读取仓库索引 Control/registry/repos.csv；通过文件处理应用打开。
3. 查找 Secure/TOTP/Authenticator.kdbx；交给 KeePassDX 打开，不导出明文。
4. 启动 com.appshub.bettbox Android 程序（若安装）。
5. 查看共享根目录；分类的主控入口与平台差异说明。
6. 采用 Material 3 Compose，Android 12+ 适配 Monet 动态配色。

尚未实现：
* OpenUUYC 原生 Android 主控。当前 fork 是 Windows-only，必须移植协议和渲染/输入。
* Android 原生 Codex；需要 aarch64-linux-android 构建、PTY/沙箱等。
* SSH 客户端（用户自行维护）。
* 内部文件编辑器（本版交给其他 app），磁盘写入测试。

构建环境：JDK 17 + Android SDK 36 + Gradle 8.14.x，命令 gradle :app:assembleDebug。
期望输出 app/build/outputs/apk/debug/app-debug.apk。
如需在 Github Actions 编译，工作流位于仓库的 .github/workflows/android-companion.yml。

安全：无 INTERNET、无 Root、无 Accessibility、无 QUERY_ALL_PACKAGES、无 MANAGE_EXTERNAL_STORAGE 权限。
仅用户通过文件选择器授权的共享目录可访问。KDBX 密码由 KeePassDX 处理。
Android 本地 PackageManager 可查到已安装 Bettbox；应用不存储远程账户信息。

注意：这套 Android 文件夹是 APK 源码，不是已编译 APK；成功构建/实机运行以 CI 和用户设备测试为准。
