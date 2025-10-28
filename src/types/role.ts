export interface Role {
  id: string
  title: string
  description: string
  prompt: string
  is_system: boolean
  created_at: Date
  updated_at: Date
}

export interface CreateRoleRequest {
  title: string
  description: string
  prompt: string
}

export interface UpdateRoleRequest {
  id: string
  title: string
  description: string
  prompt: string
}
