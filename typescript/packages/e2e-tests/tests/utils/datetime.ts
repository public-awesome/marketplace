export const getExpirationString = (expiration: number): string => {
  const now = new Date()
  now.setDate(now.getDate() + expiration)

  let padding = 0
  switch (expiration) {
    case 1:
      padding = 20 * 60 * 1_000_000 // +20 minutes (clock drift on chain can be higher)
      break
    case 180:
      padding = 20 * 60 * 1_000_000 * -1 // -20 minutes (clock drift on chain can be higher)
      break
  }

  const expires = (now.getTime() * 1_000_000 + padding).toString()
  return expires
}
