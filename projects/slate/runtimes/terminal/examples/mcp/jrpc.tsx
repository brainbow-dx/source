// deno-lint-ignore-file
// @$struct<C>()
export class JrpcEventStream extends ReadableStream<Uint8Array> {
	public readonly id?: number;

	public name?: string = undefined;

	public readonly tickSpeed?: number = 1000;

	private _isCancelled = false;

	private _startedAt?: number;

	// @$hidden(true)
	private _tickInterval?: number;

	// @$locked(true)
	private _encoder = new TextEncoder();

	constructor(options?: Partial<JrpcEventStream>) {
		super(); // TODO: Pass params for the stream.
		Object.assign(this, options);
	}

	// @$method(true)
	public start(controller: ReadableStreamDefaultController<Uint8Array>): void {
		this._startedAt = Date.now();
		this._tickInterval = setInterval(
			(_) => this.check(controller),
			this.tickSpeed,
		);
		controller.enqueue(this.encode({ message: "connected" }));
	}

	// @$method(true)
	public override async cancel(reason?: any): Promise<void> {
		console.info(`Exit Reason:`, typeof reason, reason);
		clearInterval(this._tickInterval);
		this._isCancelled = true;
	}

	// @$method(true)
	private encode<O extends object>(response: O): Uint8Array {
		const responseData = JSON.stringify(response);
		const message = `data:${responseData}\r\n\r\n`;
		return this._encoder.encode(message);
	}

	// @$method(true)
	private check(controller: ReadableStreamDefaultController<Uint8Array>): void {
		if (this._startedAt !== undefined) {
			const elapsedDuration = this._startedAt - Date.now();

			if (elapsedDuration >= 5000) {
				this.cancel();

				controller.enqueue(this.encode({ message: "closing" }));
				controller.close();
			}
		}
	}
}
