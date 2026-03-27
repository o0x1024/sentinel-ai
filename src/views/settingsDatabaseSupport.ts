export const applyDatabaseTypeDefaults = (settings: any, dbType: string) => {
  if (dbType === 'postgresql') {
    settings.database.port = 5432
    settings.database.host = settings.database.host || 'localhost'
  } else if (dbType === 'mysql') {
    settings.database.port = 3306
    settings.database.host = settings.database.host || 'localhost'
  } else if (dbType === 'sqlite') {
    settings.database.host = ''
    settings.database.port = 0
    settings.database.username = ''
    settings.database.password = ''
  }
}
