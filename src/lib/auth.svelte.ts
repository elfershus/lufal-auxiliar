class AuthState {
	unlocked = $state(false);

	unlock(password: string): boolean {
		const now = new Date();
		const day = now.getDate();
		const month = now.getMonth() + 1;
		const expected = `${day * 2}${month * 3}${day * 2}`;
		if (password === expected) {
			this.unlocked = true;
			return true;
		}
		return false;
	}
}

export const auth = new AuthState();
