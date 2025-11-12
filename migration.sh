#!/bin/bash

# Services 迁移 - 自动化脚本
# 使用方法: bash migration.sh [phase]

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICES_DIR="$REPO_ROOT/src-tauri/src/services"
SENTINEL_SERVICES="$REPO_ROOT/src-tauri/sentinel-services/src"
LOG_FILE="$REPO_ROOT/.migration_log"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

success() {
    echo -e "${GREEN}✓ $1${NC}" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}✗ $1${NC}" | tee -a "$LOG_FILE"
    exit 1
}

warning() {
    echo -e "${YELLOW}⚠ $1${NC}" | tee -a "$LOG_FILE"
}

# 初始化
init() {
    log "初始化迁移环境..."
    
    cd "$REPO_ROOT"
    
    # 检查 git
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        error "不在 Git 仓库中"
    fi
    
    # 创建日志文件
    > "$LOG_FILE"
    
    log "检查当前分支..."
    current_branch=$(git rev-parse --abbrev-ref HEAD)
    
    if [ "$current_branch" != "feature/services-migration" ]; then
        warning "当前分支: $current_branch"
        read -p "是否创建 feature/services-migration 分支? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            git checkout -b feature/services-migration
            success "已创建分支 feature/services-migration"
        fi
    fi
    
    success "初始化完成"
}

# Phase 1: 准备
phase_1_prepare() {
    log "开始 Phase 1: 准备..."
    
    cd "$REPO_ROOT/src-tauri"
    
    # 验证 Cargo.toml 中的工作区成员
    if ! grep -q "sentinel-services" Cargo.toml; then
        error "Cargo.toml 中未找到 sentinel-services"
    fi
    
    success "Cargo.toml 已配置"
    
    # 创建目录结构
    log "创建目录结构..."
    mkdir -p "$SENTINEL_SERVICES"/{database,ai,mcp,scan}
    success "目录结构已创建"
    
    # 创建 lib.rs
    if [ ! -f "$SENTINEL_SERVICES/lib.rs" ]; then
        log "创建 lib.rs..."
        touch "$SENTINEL_SERVICES/lib.rs"
        success "lib.rs 已创建"
    fi
    
    # 编译检查
    log "执行编译检查..."
    if ! cargo build --lib -p sentinel-services 2>&1 | tee -a "$LOG_FILE"; then
        warning "编译失败，这是预期的 (文件还未复制)"
    fi
    
    success "Phase 1 完成"
}

# Phase 2: 迁移独立服务
phase_2_independent() {
    log "开始 Phase 2: 迁移独立服务..."
    
    cd "$REPO_ROOT/src-tauri"
    
    local files=(
        "message_emitter.rs"
        "rag_service_impl.rs"
        "tool_manager_impl.rs"
        "asset_service.rs"
        "prompt_db.rs"
        "prompt_service.rs"
        "performance.rs"
        "dictionary.rs"
    )
    
    for file in "${files[@]}"; do
        if [ -f "$SERVICES_DIR/$file" ]; then
            log "复制 $file..."
            cp "$SERVICES_DIR/$file" "$SENTINEL_SERVICES/"
            success "已复制 $file"
        else
            warning "未找到 $file"
        fi
    done
    
    # 编译检查
    log "执行编译检查..."
    if cargo build --lib -p sentinel-services 2>&1 | tail -20 | tee -a "$LOG_FILE" | grep -q "error"; then
        warning "编译有错误，请查看上述信息并修复"
    else
        success "Phase 2 编译成功"
    fi
    
    success "Phase 2 完成"
}

# Phase 3: 迁移基础服务
phase_3_basic() {
    log "开始 Phase 3: 迁移基础服务..."
    
    cd "$REPO_ROOT/src-tauri"
    
    # 复制 scan_session.rs
    if [ -f "$SERVICES_DIR/scan_session.rs" ]; then
        log "复制 scan_session.rs..."
        cp "$SERVICES_DIR/scan_session.rs" "$SENTINEL_SERVICES/"
        success "已复制 scan_session.rs"
    fi
    
    # 修复导入
    log "修复导入..."
    sed -i '' 's/use crate::services::database/use crate::database/g' "$SENTINEL_SERVICES/scan_session.rs"
    success "导入已修复"
    
    # 编译检查
    log "执行编译检查..."
    cargo build --lib -p sentinel-services 2>&1 | tail -20 | tee -a "$LOG_FILE"
    
    success "Phase 3 完成"
}

# Phase 4: 迁移中等复杂服务
phase_4_medium() {
    log "开始 Phase 4: 迁移中等复杂服务..."
    
    cd "$REPO_ROOT/src-tauri"
    
    local files=("mcp.rs" "vulnerability.rs")
    
    for file in "${files[@]}"; do
        if [ -f "$SERVICES_DIR/$file" ]; then
            log "复制 $file..."
            cp "$SERVICES_DIR/$file" "$SENTINEL_SERVICES/"
            
            # 修复导入
            log "修复 $file 中的导入..."
            sed -i '' 's/use crate::services::/use crate::/g' "$SENTINEL_SERVICES/$file"
            
            success "已处理 $file"
        fi
    done
    
    # 编译检查
    log "执行编译检查..."
    cargo build --lib -p sentinel-services 2>&1 | tail -20 | tee -a "$LOG_FILE"
    
    success "Phase 4 完成"
}

# Phase 5: 迁移核心复杂服务
phase_5_complex() {
    log "开始 Phase 5: 迁移核心复杂服务..."
    
    cd "$REPO_ROOT/src-tauri"
    
    # 创建模块目录
    mkdir -p "$SENTINEL_SERVICES/scan"
    mkdir -p "$SENTINEL_SERVICES/ai"
    
    # 复制 scan.rs
    if [ -f "$SERVICES_DIR/scan.rs" ]; then
        log "复制 scan.rs..."
        cp "$SERVICES_DIR/scan.rs" "$SENTINEL_SERVICES/scan/mod.rs"
        sed -i '' 's/use crate::services::/use crate::/g' "$SENTINEL_SERVICES/scan/mod.rs"
        success "已处理 scan.rs"
    fi
    
    # 复制 ai.rs
    if [ -f "$SERVICES_DIR/ai.rs" ]; then
        log "复制 ai.rs..."
        cp "$SERVICES_DIR/ai.rs" "$SENTINEL_SERVICES/ai/mod.rs"
        sed -i '' 's/use crate::services::/use crate::/g' "$SENTINEL_SERVICES/ai/mod.rs"
        success "已处理 ai.rs"
    fi
    
    # 编译检查
    log "执行编译检查..."
    cargo build --lib -p sentinel-services 2>&1 | tail -20 | tee -a "$LOG_FILE"
    
    success "Phase 5 完成"
}

# Phase 6: 迁移最复杂的基础服务
phase_6_database() {
    log "开始 Phase 6: 迁移 database.rs..."
    
    cd "$REPO_ROOT/src-tauri"
    
    mkdir -p "$SENTINEL_SERVICES/database"
    
    if [ -f "$SERVICES_DIR/database.rs" ]; then
        log "复制 database.rs..."
        cp "$SERVICES_DIR/database.rs" "$SENTINEL_SERVICES/database/mod.rs"
        
        log "修复导入..."
        sed -i '' 's/use crate::services::/use crate::/g' "$SENTINEL_SERVICES/database/mod.rs"
        sed -i '' 's/use crate::models/use sentinel_core::models/g' "$SENTINEL_SERVICES/database/mod.rs"
        
        success "已处理 database.rs"
    fi
    
    # 编译检查
    log "执行编译检查..."
    cargo build --lib -p sentinel-services 2>&1 | tail -50 | tee -a "$LOG_FILE"
    
    success "Phase 6 完成"
}

# Phase 7: 集成测试
phase_7_integration() {
    log "开始 Phase 7: 集成测试..."
    
    cd "$REPO_ROOT/src-tauri"
    
    # 删除原始文件
    warning "准备删除 src/services 目录..."
    read -p "确定要删除 src/services 目录吗? (yes/no) " -r
    if [ "$REPLY" = "yes" ]; then
        rm -rf "$SERVICES_DIR"
        success "已删除 src/services"
    else
        error "取消删除操作"
    fi
    
    # 编译整个项目
    log "编译整个项目..."
    if cargo build --all 2>&1 | tee -a "$LOG_FILE"; then
        success "编译成功"
    else
        error "编译失败，请查看上述错误信息"
    fi
    
    # 运行测试
    log "运行测试..."
    if cargo test --lib 2>&1 | tee -a "$LOG_FILE"; then
        success "测试通过"
    else
        warning "某些测试失败"
    fi
    
    success "Phase 7 完成"
}

# 显示帮助
show_help() {
    cat << EOF
Services 迁移自动化脚本

用法: $0 [命令]

命令:
  init              初始化迁移环境
  phase1            执行 Phase 1 (准备)
  phase2            执行 Phase 2 (独立服务)
  phase3            执行 Phase 3 (基础服务)
  phase4            执行 Phase 4 (中等复杂)
  phase5            执行 Phase 5 (核心复杂)
  phase6            执行 Phase 6 (数据库)
  phase7            执行 Phase 7 (集成)
  all               执行所有 Phase (需要确认)
  status            显示迁移状态
  log               显示迁移日志
  help              显示此帮助信息

示例:
  bash migration.sh init
  bash migration.sh phase2
  bash migration.sh all
EOF
}

# 显示状态
show_status() {
    log "迁移状态检查..."
    
    cd "$REPO_ROOT/src-tauri"
    
    # 检查 sentinel-services 是否存在
    if [ -d "sentinel-services" ]; then
        success "sentinel-services crate 已存在"
        log "文件数: $(find sentinel-services/src -name '*.rs' | wc -l)"
    else
        warning "sentinel-services crate 不存在"
    fi
    
    # 检查原始服务是否存在
    if [ -d "src/services" ]; then
        warning "src/services 仍然存在"
        log "文件数: $(find src/services -name '*.rs' | wc -l)"
    else
        success "src/services 已删除"
    fi
    
    # 编译状态
    if cargo build --lib -p sentinel-services 2>&1 | grep -q "Finished"; then
        success "sentinel-services 编译成功"
    else
        warning "sentinel-services 编译有问题"
    fi
}

# 显示日志
show_log() {
    if [ -f "$LOG_FILE" ]; then
        less "$LOG_FILE"
    else
        error "日志文件不存在"
    fi
}

# 执行所有 Phase
run_all() {
    warning "即将执行所有 Phase 的迁移..."
    read -p "确定继续吗? (yes/no) " -r
    if [ "$REPLY" != "yes" ]; then
        error "取消操作"
    fi
    
    phase_1_prepare
    phase_2_independent
    phase_3_basic
    phase_4_medium
    phase_5_complex
    phase_6_database
    phase_7_integration
    
    success "所有 Phase 已完成！"
}

# 主程序
main() {
    case "${1:-help}" in
        init)
            init
            ;;
        phase1)
            init
            phase_1_prepare
            ;;
        phase2)
            init
            phase_2_independent
            ;;
        phase3)
            init
            phase_3_basic
            ;;
        phase4)
            init
            phase_4_medium
            ;;
        phase5)
            init
            phase_5_complex
            ;;
        phase6)
            init
            phase_6_database
            ;;
        phase7)
            init
            phase_7_integration
            ;;
        all)
            init
            run_all
            ;;
        status)
            show_status
            ;;
        log)
            show_log
            ;;
        help)
            show_help
            ;;
        *)
            error "未知命令: $1"
            ;;
    esac
}

# 运行主程序
main "$@"
