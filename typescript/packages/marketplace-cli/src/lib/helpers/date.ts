export const ISOToNano = (ISO: string) => {
  return (new Date(ISO).getTime() * 1_000_000).toString()
}

export const ISOToSeconds = (ISO: string) => {
  return Math.floor(new Date(ISO).getTime() / 1000)
}

export const isISODate = (ISO: string) => {
  if (!/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d{3}Z/.test(ISO)) return false
  var d = new Date(ISO)
  return d.toISOString() === ISO
}
