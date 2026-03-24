#!/usr/bin/env python3
"""Extract and analyze i18n keys from the codebase."""

import os
import re
from pathlib import Path
from collections import defaultdict

# Get all translation keys from code
def extract_keys_from_code(src_dir):
    """Extract all t() keys from Vue and TypeScript files."""
    keys = set()
    files_with_keys = defaultdict(list)
    
    # Regex pattern to match t('key') or t("key") or t(`key`)
    pattern = r"t\(['\"`]([^'\"` ,)]+)['\"`]\)"
    
    for root, dirs, files in os.walk(src_dir):
        # Skip node_modules and test dirs
        dirs[:] = [d for d in dirs if d not in ['node_modules', '.next', 'dist', '__pycache__']]
        
        for file in files:
            if file.endswith(('.vue', '.ts', '.tsx')) and not file.endswith('.d.ts'):
                filepath = os.path.join(root, file)
                try:
                    with open(filepath, 'r', encoding='utf-8') as f:
                        content = f.read()
                        matches = re.findall(pattern, content)
                        for match in matches:
                            keys.add(match)
                            files_with_keys[match].append(filepath.replace(src_dir, ''))
                except Exception as e:
                    print(f"Error reading {filepath}: {e}")
    
    return keys, files_with_keys


def extract_keys_from_locale(locale_file):
    """Extract all top-level and nested keys from a locale file."""
    keys = set()
    
    try:
        with open(locale_file, 'r', encoding='utf-8') as f:
            content = f.read()
            
            # Extract variable names from 'import X from ...' lines
            import_pattern = r"import\s+(\w+)\s+from\s+'\./"
            imports = re.findall(import_pattern, content)
            
            # Find the export default object
            # Look for keys directly defined in the export
            export_pattern = r"\b(\w+):\s*{[^}]*}(?:\s*,|\s*})"
            exports_in_main = re.findall(export_pattern, content)
            
            # Add import variables (they represent submodules)
            for imp in imports:
                keys.add(imp)
            
            # Add direct exports
            for exp in exports_in_main:
                keys.add(exp)
            
            # Also extract nested keys in inline objects
            inline_obj_pattern = r"(\w+):\s*\{[^}]*?\b(\w+):"
            inline_matches = re.findall(inline_obj_pattern, content)
            for parent, child in inline_matches:
                keys.add(f"{parent}.{child}")
    except Exception as e:
        print(f"Error reading {locale_file}: {e}")
    
    return keys


def load_locale_recursively(locale_file):
    """Load all keys from a locale file, including imported modules."""
    keys_found = {}
    
    try:
        with open(locale_file, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Parse keys in the current file
        # First, get the directory of the locale file
        locale_dir = os.path.dirname(locale_file)
        
        # Extract imports
        import_lines = re.findall(r"import\s+(\w+)\s+from\s+'([^']+)'", content)
        
        for var_name, import_path in import_lines:
            # Resolve the import path
            full_path = os.path.join(locale_dir, import_path.replace('./', ''))
            if not full_path.endswith('.ts'):
                full_path += '.ts'
            
            if os.path.exists(full_path):
                # Recursively load the imported module
                sub_keys = load_nested_module(full_path)
                for key, value in sub_keys.items():
                    keys_found[f"{var_name}.{key}"] = value
        
        # Extract inline object keys
        # Find lines with key: value pattern
        lines = content.split('\n')
        for line in lines:
            # Skip comments and imports
            if line.strip().startswith('//') or 'import' in line:
                continue
            
            # Find simple key: value patterns
            match = re.match(r'\s*(\w+):\s*[\'"]?([^\'"]+)[\'"]?,?\s*$', line)
            if match:
                key = match.group(1)
                keys_found[key] = match.group(2)
        
    except Exception as e:
        print(f"Error loading {locale_file}: {e}")
    
    return keys_found


def load_nested_module(module_path):
    """Load keys from a nested i18n module."""
    keys = {}
    try:
        with open(module_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Extract the default export object
        export_match = re.search(r'export\s+default\s+({[\s\S]*})', content)
        if export_match:
            export_obj = export_match.group(1)
            # Simple parsing of the object
            matches = re.findall(r'(\w+):\s*[\'"]?([^\'"]+)[\'"]?,?', export_obj)
            for key, value in matches:
                keys[key] = value
    except Exception as e:
        print(f"Error loading nested module {module_path}: {e}")
    
    return keys


def extract_all_locale_keys(locale_file):
    """Extract all possible translation keys from locale files."""
    all_keys = set()
    
    try:
        with open(locale_file, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Get locale directory
        locale_dir = os.path.dirname(locale_file)
        
        # Extract all imported module names and their files
        import_lines = re.findall(r"import\s+(\w+)\s+from\s+'([^']+)'", content)
        
        for var_name, import_path in import_lines:
            # Resolve the import path
            full_path = os.path.join(locale_dir, import_path.replace('./', ''))
            if not full_path.endswith('.ts'):
                full_path += '.ts'
            
            if os.path.exists(full_path):
                # Load keys from the module recursively
                module_keys = load_module_keys(full_path)
                for key in module_keys:
                    all_keys.add(f"{var_name}.{key}")
        
        # Also add keys directly in the export
        with open(locale_file, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Find all top-level keys in the export default   
        export_pattern = r"export\s+default\s+\{([\s\S]*?)\n\}"
        export_match = re.search(export_pattern, content)
        if export_match:
            export_content = export_match.group(1)
            # Get all first-level keys
            key_pattern = r"(\w+):\s*"
            for key in re.findall(key_pattern, export_content):
                all_keys.add(key)
                # For complex nested objects, try to get sub-keys
                nested_obj = re.search(rf"{key}:\s*\{{([^}}]*)\}}", export_content)
                if nested_obj:
                    nested_content = nested_obj.group(1)
                    for subkey in re.findall(r"(\w+):", nested_content):
                        all_keys.add(f"{key}.{subkey}")
    
    except Exception as e:
        print(f"Error processing {locale_file}: {e}")
    
    return all_keys


def load_module_keys(module_path):
    """Recursively load all keys from a module."""
    keys = set()
    try:
        with open(module_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Extract export default object
        pattern = r"export\s+default\s+\{([\s\S]*?)\n\}"
        match = re.search(pattern, content)
        if match:
            obj_content = match.group(1)
            # Get all top-level keys
            for key in re.findall(r"(\w+):", obj_content):
                keys.add(key)
                # Look for nested objects
                nested = re.search(rf"{key}:\s*\{{([^}}]*)\}}", obj_content)
                if nested:
                    nested_keys = re.findall(r"(\w+):", nested.group(1))
                    for nkey in nested_keys:
                        keys.add(f"{key}.{nkey}")
    except Exception as e:
        print(f"Error with {module_path}: {e}")
    
    return keys


# Main execution
if __name__ == '__main__':
    src_dir = '/Users/like/code/sentinel-ai/src'
    
    # Extract keys from code
    print("Extracting keys from code...")
    code_keys, files_with_keys = extract_keys_from_code(src_dir)
    print(f"Found {len(code_keys)} unique keys in code")
    
    # Extract keys from locale files
    print("\nExtracting keys from locale files...")
    zh_file = '/Users/like/code/sentinel-ai/src/i18n/locales/zh.ts'
    en_file = '/Users/like/code/sentinel-ai/src/i18n/locales/en.ts'
    
    zh_keys = extract_all_locale_keys(zh_file)
    en_keys = extract_all_locale_keys(en_file)
    
    print(f"Found {len(zh_keys)} unique keys in zh.ts")
    print(f"Found {len(en_keys)} unique keys in en.ts")
    
    # Find missing keys
    missing_in_zh = code_keys - zh_keys
    missing_in_en = code_keys - en_keys
    
    # Output results
    print("\n" + "="*80)
    print("MISSING TRANSLATION KEYS")
    print("="*80)
    
    if missing_in_zh:
        print(f"\n[{len(missing_in_zh)}] Missing in zh.ts:")
        for key in sorted(missing_in_zh):
            print(f"  - {key}")
    else:
        print("\n✓ All keys present in zh.ts")
    
    if missing_in_en:
        print(f"\n[{len(missing_in_en)}] Missing in en.ts:")
        for key in sorted(missing_in_en):
            print(f"  - {key}")
    else:
        print("\n✓ All keys present in en.ts")
    
    print("\n" + "="*80)
    print("GROUPED BY NAMESPACE")
    print("="*80)
    
    # Group missing keys by namespace
    missing_combined = missing_in_zh | missing_in_en
    by_namespace = defaultdict(list)
    
    for key in sorted(missing_combined):
        if '.' in key:
            namespace = key.split('.')[0]
            by_namespace[namespace].append(key)
        else:
            by_namespace['[root]'].append(key)
    
    for namespace in sorted(by_namespace.keys()):
        keys = by_namespace[namespace]
        zh_missing = [k for k in keys if k in missing_in_zh]
        en_missing = [k for k in keys if k in missing_in_en]
        
        print(f"\n### {namespace}")
        if zh_missing:
            print(f"  Missing in zh.ts ({len(zh_missing)}):")
            for key in sorted(zh_missing):
                print(f"    - {key}")
        if en_missing:
            print(f"  Missing in en.ts ({len(en_missing)}):")
            for key in sorted(en_missing):
                print(f"    - {key}")
