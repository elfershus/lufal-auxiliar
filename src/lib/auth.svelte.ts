const PASSWORD = 'wombocombo69';

class AuthState {
	unlocked = $state(false);

	unlock(password: string): boolean {
		if (password === PASSWORD) {
			this.unlocked = true;
			return true;
		}
		return false;
	}
}

export const auth = new AuthState();
