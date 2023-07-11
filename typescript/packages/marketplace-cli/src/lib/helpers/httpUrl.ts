export const isValidHttpUrl = (uri: string) => {
  let url
  try {
    url = new URL(uri)
  } catch (_) {
    return false
  }
  return url.protocol === 'http:' || url.protocol === 'https:'
}
