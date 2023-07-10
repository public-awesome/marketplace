export const toRawAmount = (amount: number, decimals: number = 6) => {
  return (amount * 10 ** decimals).toFixed()
}

export const fromRawAmount = (amount: string, decimals: number = 6) =>
  parseFloat(amount) / 10 ** decimals
