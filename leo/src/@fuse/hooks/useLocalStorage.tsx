function useLocalStorage<T>(key: string) {
	function getValue() {
		try {
			const item = window.localStorage.getItem(key);
			return item ? (JSON.parse(item) as T) : null;
		} catch (error) {
			console.error(error);
			return null;
		}
	}

	const setValue = (value: T) => {
		try {
			window.localStorage.setItem(key, JSON.stringify(value));
		} catch (error) {
			console.error(error);
		}
	};

	const removeValue = () => {
		window.localStorage.removeItem(key);
	};

	return { value: getValue(), setValue, getValue, removeValue };
}

export default useLocalStorage;
