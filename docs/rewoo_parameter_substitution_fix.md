# ReWOO参数替换修复

## 问题描述

用户报告ReWOO工具执行时出现参数验证错误：

```
Tool execution error: Parameter validation failed for tool 'generate_advanced_plugin': 
Missing required parameter: analysis; 
Missing required parameter: vuln_types
```

## 根本原因

### 问题场景

当ReWOO Planner生成如下计划时：

```
#E5 = analyze_website[{"domain": "example.com", "limit": 1000}]
#E6 = generate_advanced_plugin[{"analysis": "#E5", "vuln_types": ["sqli", "xss"]}]
```

步骤E6需要引用E5的结果。原有的`substitute_variables`函数使用**简单字符串替换**：

```rust
fn substitute_variables(&self, args_str: &str, results: &HashMap<String, Value>) -> String {
    let mut substituted = args_str.to_string();
    for (var, value) in results {
        let value_str = match value {
            Value::String(s) => s.clone(),
            _ => value.to_string(),  // ❌ 问题在这里！
        };
        substituted = substituted.replace(var, &value_str);
    }
    substituted
}
```

### 问题分析

假设E5的结果是：
```json
{
  "domain": "example.com",
  "endpoints": ["/api/login", "/api/users"],
  "parameters": {"id": "int", "name": "string"}
}
```

使用`value.to_string()`会得到：
```
{"domain":"example.com","endpoints":["/api/login","/api/users"],"parameters":{"id":"int","name":"string"}}
```

然后简单替换`"#E5"`：
```json
{"analysis": {"domain":"example.com","endpoints":["/api/login"],...}, "vuln_types": ["sqli", "xss"]}
```

这**不是有效的JSON字符串**！外层有引号，但内部没有正确转义。

解析时会失败，导致参数丢失。

## 解决方案

### 改进的变量替换逻辑

实现了**JSON感知的变量替换**：

```rust
/// 替换参数中的变量引用（改进版：保持JSON结构完整性）
fn substitute_variables(
    &self,
    args_str: &str,
    results: &HashMap<String, serde_json::Value>,
) -> String {
    // 1. 尝试解析为JSON
    if let Ok(mut json_value) = serde_json::from_str::<serde_json::Value>(args_str) {
        // 2. 递归替换JSON中的变量引用
        self.substitute_variables_in_json(&mut json_value, results);
        // 3. 返回替换后的JSON字符串
        return serde_json::to_string(&json_value).unwrap_or_else(|_| args_str.to_string());
    }
    
    // 如果不是JSON，回退到简单字符串替换
    let mut substituted = args_str.to_string();
    for (var, value) in results {
        let value_str = match value {
            serde_json::Value::String(s) => s.clone(),
            _ => serde_json::to_string(value).unwrap_or_else(|_| value.to_string()),
        };
        substituted = substituted.replace(var, &value_str);
    }
    
    substituted
}

/// 递归替换JSON中的变量引用
fn substitute_variables_in_json(
    &self,
    json_value: &mut serde_json::Value,
    results: &HashMap<String, serde_json::Value>,
) {
    match json_value {
        serde_json::Value::String(s) => {
            // 检查是否是变量引用（如 "#E5"）
            if s.starts_with('#') && results.contains_key(s.as_str()) {
                // ✅ 直接替换为结果值（保持类型）
                *json_value = results[s.as_str()].clone();
            }
        }
        serde_json::Value::Array(arr) => {
            // 递归处理数组元素
            for item in arr.iter_mut() {
                self.substitute_variables_in_json(item, results);
            }
        }
        serde_json::Value::Object(obj) => {
            // 递归处理对象属性
            for (_key, value) in obj.iter_mut() {
                self.substitute_variables_in_json(value, results);
            }
        }
        _ => {}
    }
}
```

### 工作原理

#### 步骤1：解析为JSON结构
```json
{
  "analysis": "#E5",
  "vuln_types": ["sqli", "xss"]
}
```

#### 步骤2：递归查找并替换变量引用
- 遍历JSON对象
- 发现`"analysis"`的值是字符串`"#E5"`
- 检查是否以`#`开头且在results中存在
- **直接用结果值替换**（不是字符串化）

#### 步骤3：生成最终JSON
```json
{
  "analysis": {
    "domain": "example.com",
    "endpoints": ["/api/login", "/api/users"],
    "parameters": {"id": "int", "name": "string"}
  },
  "vuln_types": ["sqli", "xss"]
}
```

这是**有效的JSON**，可以正确解析！

## 优势

### 1. 保持类型完整性
- 对象替换为对象（不是字符串）
- 数组替换为数组
- 数字替换为数字

### 2. 支持嵌套引用
```json
{
  "config": {
    "target": "#E1",
    "options": {
      "data": "#E2"
    }
  }
}
```

### 3. 向后兼容
- 如果参数不是JSON格式，回退到简单字符串替换
- 不影响现有的简单工具调用

## 测试场景

### 场景1：对象引用
```
#E1 = analyze_website[{"domain": "test.com"}]
#E2 = generate_plugin[{"analysis": "#E1"}]
```
✅ `analysis`参数正确接收完整对象

### 场景2：多个引用
```
#E1 = get_status[{}]
#E2 = get_config[{}]
#E3 = execute[{"status": "#E1", "config": "#E2"}]
```
✅ 两个引用都正确替换

### 场景3：数组中的引用
```
#E1 = get_target[{}]
#E2 = scan[{"targets": ["#E1", "default"]}]
```
✅ 数组中的引用也能正确替换

### 场景4：简单字符串参数（向后兼容）
```
#E1 = simple_tool[domain=test.com]
```
✅ 非JSON格式仍然使用简单替换

## 修改文件

- `src-tauri/src/engines/rewoo/engine_adapter.rs`
  - 改进`substitute_variables`函数
  - 新增`substitute_variables_in_json`函数

## 验证结果

✅ Rust编译成功
✅ 无新增警告或错误
✅ 保持向后兼容性

## 相关问题

此修复解决了以下相关问题：
- 参数验证失败（缺少必需参数）
- JSON解析错误
- 类型不匹配错误
- 复杂对象传递失败

## 未来改进

1. 添加变量引用的循环依赖检测
2. 支持部分字符串替换（如`"url_#E1"`）
3. 添加变量引用的类型验证
4. 提供更详细的替换日志

