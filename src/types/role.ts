export interface Role {
  id: string
  title: string
  description: string
  prompt: string
  capabilities: string[]
  is_system: boolean
  created_at: Date
  updated_at: Date
}

export interface CreateRoleRequest {
  title: string
  description: string
  prompt: string
  capabilities?: string[]
}

export interface UpdateRoleRequest {
  id: string
  title: string
  description: string
  prompt: string
  capabilities?: string[]
}
