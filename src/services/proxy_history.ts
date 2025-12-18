/**
 * 代理历史服务
 * 用于与后端内存缓存交互，获取 HTTP 和 WebSocket 历史记录
 */
import { invoke } from '@tauri-apps/api/core';

// HTTP 请求记录
export interface HttpRequestRecord {
    id: number;
    url: string;
    host: string;
    protocol: string;
    method: string;
    status_code: number;
    request_headers?: string;
    request_body?: string;
    response_headers?: string;
    response_body?: string;
    response_size: number;
    response_time: number;
    timestamp: string;
    ip?: string;
    listener?: string;
    extension?: string;
    title?: string;
    mime_type?: string;
}

// WebSocket 连接状态
export type WebSocketConnectionStatus = 'open' | 'closed' | 'error';

// WebSocket 消息方向
export type WebSocketDirection = 'send' | 'receive';

// WebSocket 消息类型
export type WebSocketMessageType = 'text' | 'binary' | 'ping' | 'pong' | 'close';

// WebSocket 连接记录
export interface WebSocketConnectionRecord {
    id: string;
    url: string;
    host: string;
    protocol: string;
    request_headers?: string;
    response_headers?: string;
    status: WebSocketConnectionStatus;
    opened_at: string;
    closed_at?: string;
    close_code?: number;
    close_reason?: string;
    message_ids: number[];
}

// WebSocket 消息记录
export interface WebSocketMessageRecord {
    id: number;
    connection_id: string;
    direction: WebSocketDirection;
    message_type: WebSocketMessageType;
    content?: string;
    content_length: number;
    timestamp: string;
}

// 历史缓存统计
export interface HistoryCacheStats {
    http_count: number;
    ws_connection_count: number;
    ws_message_count: number;
    max_http_requests: number;
    max_ws_connections: number;
    max_messages_per_connection: number;
}

// 通用 API 响应
interface ApiResponse<T> {
    success: boolean;
    data?: T;
    error?: string;
}

/**
 * 获取 HTTP 请求列表
 */
export async function listHttpRequests(options?: {
    limit?: number;
    offset?: number;
    protocol?: string;
    method?: string;
    host?: string;
    statusCodeMin?: number;
    statusCodeMax?: number;
}): Promise<HttpRequestRecord[]> {
    const response = await invoke<ApiResponse<HttpRequestRecord[]>>('list_proxy_requests', {
        limit: options?.limit ?? 100,
        offset: options?.offset ?? 0,
        protocol: options?.protocol,
        method: options?.method,
        host: options?.host,
        statusCodeMin: options?.statusCodeMin,
        statusCodeMax: options?.statusCodeMax,
    });

    if (response.success && response.data) {
        return response.data;
    }
    throw new Error(response.error || 'Failed to list HTTP requests');
}

/**
 * 获取单个 HTTP 请求详情
 */
export async function getHttpRequest(id: number): Promise<HttpRequestRecord | null> {
    const response = await invoke<ApiResponse<HttpRequestRecord | null>>('get_proxy_request', { id });

    if (response.success) {
        return response.data ?? null;
    }
    throw new Error(response.error || 'Failed to get HTTP request');
}

/**
 * 清空 HTTP 请求历史
 */
export async function clearHttpRequests(): Promise<number> {
    const response = await invoke<ApiResponse<number>>('clear_proxy_requests');

    if (response.success && response.data !== undefined) {
        return response.data;
    }
    throw new Error(response.error || 'Failed to clear HTTP requests');
}

/**
 * 统计 HTTP 请求数量
 */
export async function countHttpRequests(): Promise<number> {
    const response = await invoke<ApiResponse<number>>('count_proxy_requests', {});

    if (response.success && response.data !== undefined) {
        return response.data;
    }
    throw new Error(response.error || 'Failed to count HTTP requests');
}

/**
 * 获取 WebSocket 连接列表
 */
export async function listWebSocketConnections(options?: {
    host?: string;
    status?: string;
    limit?: number;
    offset?: number;
}): Promise<WebSocketConnectionRecord[]> {
    const response = await invoke<ApiResponse<WebSocketConnectionRecord[]>>('list_websocket_connections', {
        host: options?.host,
        status: options?.status,
        limit: options?.limit,
        offset: options?.offset,
    });

    if (response.success && response.data) {
        return response.data;
    }
    throw new Error(response.error || 'Failed to list WebSocket connections');
}

/**
 * 获取 WebSocket 消息列表
 */
export async function listWebSocketMessages(connectionId: string, options?: {
    direction?: string;
    messageType?: string;
    limit?: number;
    offset?: number;
}): Promise<WebSocketMessageRecord[]> {
    const response = await invoke<ApiResponse<WebSocketMessageRecord[]>>('list_websocket_messages', {
        connectionId,
        direction: options?.direction,
        messageType: options?.messageType,
        limit: options?.limit,
        offset: options?.offset,
    });

    if (response.success && response.data) {
        return response.data;
    }
    throw new Error(response.error || 'Failed to list WebSocket messages');
}

/**
 * 清空 WebSocket 历史
 */
export async function clearWebSocketHistory(): Promise<number> {
    const response = await invoke<ApiResponse<number>>('clear_websocket_history');

    if (response.success && response.data !== undefined) {
        return response.data;
    }
    throw new Error(response.error || 'Failed to clear WebSocket history');
}

/**
 * 获取历史缓存统计信息
 */
export async function getHistoryStats(): Promise<HistoryCacheStats> {
    const response = await invoke<ApiResponse<HistoryCacheStats>>('get_history_stats');

    if (response.success && response.data) {
        return response.data;
    }
    throw new Error(response.error || 'Failed to get history stats');
}

/**
 * 清空所有历史记录 (HTTP + WebSocket)
 */
export async function clearAllHistory(): Promise<void> {
    const response = await invoke<ApiResponse<string>>('clear_all_history');

    if (!response.success) {
        throw new Error(response.error || 'Failed to clear all history');
    }
}
