export class CommandError extends Error {
  constructor(...args: any[]) {
    super(...args)
    this.name = this.constructor.name
  }
}

export class InvalidInput extends CommandError {
  constructor(message: string) {
    super(message)
  }
}

export class QueryError extends CommandError {
  constructor(message: string) {
    super(message)
  }
}
