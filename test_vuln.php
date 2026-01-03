<?php
// 模拟可能的漏洞模式
// 模式1: 直接eval GET参数
if (isset($_GET['cmd'])) {
    eval($_GET['cmd']);
}

// 模式2: 通过POST参数
if (isset($_POST['code'])) {
    $code = $_POST['code'];
    eval($code);
}

// 模式3: 通过COOKIE
if (isset($_COOKIE['exec'])) {
    $exec = $_COOKIE['exec'];
    system($exec);
}

// 模式4: 通过REQUEST（包含GET/POST/COOKIE）
if (isset($_REQUEST['payload'])) {
    $payload = $_REQUEST['payload'];
    assert($payload);
}

echo "Test page loaded";
?>
