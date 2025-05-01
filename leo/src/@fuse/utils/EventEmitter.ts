/**
 * The EventEmitter class is a custom implementation of an event emitter.
 * It provides methods for registering and emitting events.
 */
class EventEmitter {
	private events: Record<string, Set<(...args: unknown[]) => void>>;

	constructor() {
		this.events = {};
	}

	/**
	 * The _getEventListByName method returns the event list for a given event name.
	 * If the event list does not exist, it creates a new one.
	 *
	 */
	private _getEventListByName<T extends unknown[]>(eventName: string): Set<(...args: T) => void> {
		if (typeof this.events[eventName] === 'undefined') {
			this.events[eventName] = new Set();
		}

		return this.events[eventName];
	}

	/**
	 * The on method registers a callback function for a given event name.
	 *
	 */
	on<T extends unknown[]>(eventName: string, fn: (...args: T) => void): void {
		this._getEventListByName<T>(eventName).add(fn);
	}

	/**
	 * The once method registers a callback function for a given event name that will only be called once.
	 *
	 */
	once(eventName: string, fn: (...args: unknown[]) => void): void {
		const onceFn = (...args: unknown[]) => {
			this.removeListener(eventName, onceFn);
			fn.apply(this, args);
		};
		this.on(eventName, onceFn);
	}

	/**
	 * The emit method triggers all registered callback functions for a given event name.
	 *
	 */
	emit(eventName: string, ...args: unknown[]): void {
		this._getEventListByName(eventName).forEach((fn) => {
			fn.apply(this, args);
		});
	}

	/**
	 * The removeListener method removes a registered callback function for a given event name.
	 *
	 */
	removeListener(eventName: string, fn: (...args: unknown[]) => void): void {
		this._getEventListByName(eventName).delete(fn);
	}
}

export default EventEmitter;
