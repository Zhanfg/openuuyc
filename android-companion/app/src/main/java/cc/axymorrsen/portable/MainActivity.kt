package cc.axymorrsen.portable

import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.provider.DocumentsContract
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.documentfile.provider.DocumentFile

class MainActivity : ComponentActivity() {
    private val prefs by lazy { getSharedPreferences("portable-root-v1", MODE_PRIVATE) }
    private fun restoreRoot(): Uri? = prefs.getString("tree_uri", null)?.let(Uri::parse)
    private fun persistRoot(uri: Uri) {
        val flags = Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        try {
            contentResolver.takePersistableUriPermission(uri, flags)
        } catch (_: SecurityException) {
            contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        prefs.edit().putString("tree_uri", uri.toString()).apply()
    }
    private fun documentRoot(uri: Uri?): DocumentFile? = try {
        uri?.let { DocumentFile.fromTreeUri(this, it) }
    } catch (_: Exception) { null }
    private fun child(root: DocumentFile?, vararg names: String): DocumentFile? {
        var node = root
        for (name in names) node = node?.findFile(name)
        return node
    }
    private fun openFile(file: DocumentFile?, mime: String) {
        if (file == null || !file.isFile) {
            Toast.makeText(this, "未找到目标文件；请检查 U 盘目录", Toast.LENGTH_LONG).show()
            return
        }
        val intent = Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(file.uri, mime)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            clipData = android.content.ClipData.newUri(contentResolver, "AXY file", file.uri)
        }
        try { startActivity(Intent.createChooser(intent, "选择可读取文件的应用")) }
        catch (_: ActivityNotFoundException) {
            Toast.makeText(this, "请先安装对应的 Android 应用", Toast.LENGTH_LONG).show()
        }
    }
    private fun openBettbox() {
        val packages = arrayOf("com.appshub.bettbox", "com.appshub.bettbox.debug")
        val i = packages.firstNotNullOfOrNull { packageManager.getLaunchIntentForPackage(it) }
        if (i != null) startActivity(i) else Toast.makeText(this, "请先安装 Bettbox Android", Toast.LENGTH_LONG).show()
    }
    private fun openWeb(url: String) {
        try { startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))) }
        catch (_: ActivityNotFoundException) { Toast.makeText(this, "没有可用的浏览器", Toast.LENGTH_LONG).show() }
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            var tree by remember { mutableStateOf(restoreRoot()) }
            var refresh by remember { mutableIntStateOf(0) }
            var tab by remember { mutableIntStateOf(0) }
            val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
                if (uri != null) {
                    try { persistRoot(uri); tree = uri; refresh++ }
                    catch (e: Exception) { Toast.makeText(this, "无法保存目录权限：${e.message}", Toast.LENGTH_LONG).show() }
                }
            }
            val root = remember(tree, refresh) { documentRoot(tree) }
            val repoIndex = remember(tree, refresh) { child(root, "Control", "registry", "repos.csv") }
            val totp = remember(tree, refresh) { child(root, "Secure", "TOTP", "Authenticator.kdbx") }
            val valid = remember(tree, refresh) { root != null && child(root, "Control")?.isDirectory == true }
            MaterialTheme(colorScheme = dynamicColorScheme()) {
                Scaffold(
                    topBar = {
                        Surface(tonalElevation = 2.dp) {
                            Row(Modifier.fillMaxWidth().padding(18.dp), verticalAlignment = Alignment.CenterVertically) {
                                Icon(Icons.Outlined.Usb, null, modifier = Modifier.size(28.dp))
                                Spacer(Modifier.width(12.dp))
                                Column {
                                    Text("AXY Portable", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                                    Text("Android USB Companion · 0.1", style = MaterialTheme.typography.labelSmall)
                                }
                            }
                        }
                    },
                    bottomBar = {
                        NavigationBar {
                            NavigationBarItem(tab == 0, onClick = { tab = 0 }, icon = { Icon(Icons.Outlined.Dashboard, null) }, label = { Text("总览") })
                            NavigationBarItem(tab == 1, onClick = { tab = 1 }, icon = { Icon(Icons.Outlined.Folder, null) }, label = { Text("文件") })
                            NavigationBarItem(tab == 2, onClick = { tab = 2 }, icon = { Icon(Icons.Outlined.Devices, null) }, label = { Text("远程") })
                        }
                    }
                ) { contentPadding ->
                    LazyColumn(Modifier.fillMaxSize().padding(contentPadding), contentPadding = PaddingValues(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                        item {
                            ElevatedCard(Modifier.fillMaxWidth(), shape = RoundedCornerShape(20.dp)) {
                                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                    Row(verticalAlignment = Alignment.CenterVertically) {
                                        Icon(if (valid) Icons.Outlined.CheckCircle else Icons.Outlined.Usb, null)
                                        Spacer(Modifier.width(8.dp))
                                        Text(if (valid) "开发栈已连接" else "未连接开发栈目录", fontWeight = FontWeight.SemiBold)
                                    }
                                    Text("选择 U 盘上的 AXY-DevStack 文件夹。仅申请文件夹授权，不需要 Root 或全部文件访问权限。", style = MaterialTheme.typography.bodySmall)
                                    Button(onClick = { pick.launch(tree) }) { Text(if (valid) "重新选择 U 盘" else "选择 U 盘目录") }
                                    if (tree != null && !valid) Text("当前目录不包含 Control 文件夹，请重新选择。", color = MaterialTheme.colorScheme.error)
                                }
                            }
                        }
                        when(tab) {
                            0 -> {
                                item { Text("常用设施", style=MaterialTheme.typography.titleMedium) }
                                item { ActionCard("Bettbox 代理", "使用独立的 Android VPN 客户端", Icons.Outlined.Shield) { openBettbox() } }
                                item { ActionCard("TOTP 加密库", if(totp?.isFile == true) "打开 U 盘中的 Authenticator.kdbx" else "尚未发现 KDBX 文件", Icons.Outlined.Key) { openFile(totp, "application/x-keepass2") } }
                                item { ActionCard("仓库索引", if(repoIndex?.isFile == true) "查看 repo 清单" else "尚未找到 repos.csv", Icons.Outlined.Code) { openFile(repoIndex,"text/comma-separated-values") } }
                                item { Text("运行限制", style=MaterialTheme.typography.titleMedium) }
                                item { Text("Android 不能运行 Windows 上的 Codex.exe / OpenUUYC.exe。安卓 UU 原生主控协议和视频管线仍需单独移植。", style=MaterialTheme.typography.bodyMedium) }
                            }
                            1 -> {
                                item { Text("共享目录", style=MaterialTheme.typography.titleMedium) }
                                if (!valid) item { Text("连接 U 盘后显示开发栈目录。") }
                                else {
                                    val dirs = remember(tree, refresh) { root?.listFiles()?.sortedBy { it.name }.orEmpty() }
                                    items(dirs) { file ->
                                        ActionCard(file.name ?: "（无名称）", if(file.isDirectory) "目录 · 可交由系统文件管理器打开" else "文件", if(file.isDirectory) Icons.Outlined.Folder else Icons.Outlined.Description) {
                                            // SAF does not guarantee that providers expose a browsable directory UI.
                                            val i = Intent(Intent.ACTION_VIEW).apply { setDataAndType(file.uri, if(file.isDirectory) "vnd.android.document/directory" else (file.type ?: "*/*")); addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION) }
                                            try { startActivity(Intent.createChooser(i,"使用应用打开")) } catch (_: Exception) { Toast.makeText(this@MainActivity,"请用系统文件管理器浏览 U 盘",Toast.LENGTH_LONG).show() }
                                        }
                                    }
                                }
                            }
                            2 -> {
                                item { Text("远程访问", style=MaterialTheme.typography.titleMedium) }
                                item { ActionCard("OpenUUYC Android", "主控端移植中：Rust 协议/会话核心 + MediaCodec + 触摸输入", Icons.Outlined.DesktopWindows) { openWeb("https://github.com/Zhanfg/openuuyc") } }
                                item { ActionCard("SSH", "保留独立配置；密钥不由本应用读取或保管", Icons.Outlined.Terminal) {
                                    Toast.makeText(this@MainActivity,"SSH 由你安装的客户端维护",Toast.LENGTH_LONG).show()
                                } }
                                item { ActionCard("Bettbox", "网络链路交由系统 VPN 服务；不接管其他应用流量", Icons.Outlined.NetworkCheck) { openBettbox() } }
                                item { Text("本 APK 不提供被控服务，不请求无障碍、悬浮窗或 VPN 权限，也不收集登录令牌。", style = MaterialTheme.typography.bodySmall) }
                            }
                        }
                    }
                }
            }
        }
    }
    @Composable private fun dynamicColorScheme(): ColorScheme = if (android.os.Build.VERSION.SDK_INT >= 31) {
        if (androidx.compose.foundation.isSystemInDarkTheme()) dynamicDarkColorScheme(this) else dynamicLightColorScheme(this)
    } else if (androidx.compose.foundation.isSystemInDarkTheme()) darkColorScheme() else lightColorScheme()
}

@Composable private fun ActionCard(title: String, subtitle: String, image: androidx.compose.ui.graphics.vector.ImageVector, onClick: () -> Unit) {
    Card(onClick=onClick, modifier=Modifier.fillMaxWidth(), shape=RoundedCornerShape(18.dp)) {
        Row(Modifier.fillMaxWidth().padding(16.dp), verticalAlignment=Alignment.CenterVertically) {
            Icon(image, contentDescription=null, modifier=Modifier.size(26.dp))
            Spacer(Modifier.width(14.dp))
            Column(Modifier.weight(1f)) {
                Text(title, fontWeight=FontWeight.Medium)
                Spacer(Modifier.height(2.dp))
                Text(subtitle, style=MaterialTheme.typography.bodySmall, maxLines=3, overflow=TextOverflow.Ellipsis)
            }
            Spacer(Modifier.width(8.dp))
            Icon(Icons.Outlined.ChevronRight, null)
        }
    }
}
