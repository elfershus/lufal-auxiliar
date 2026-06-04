class AuthState {
	unlocked = $state(false);

	unlock(password: string): boolean {
		const now = new Date();
		const parts = new Intl.DateTimeFormat('es-MX', {
			timeZone: 'America/Mexico_City',
			day: 'numeric',
			month: 'numeric'
		}).formatToParts(now);
		const day   = parseInt(parts.find(p => p.type === 'day')!.value);
		const month = parseInt(parts.find(p => p.type === 'month')!.value);
		const expected = `${day * 2}${month * 3}${day * 2}`;
		if (password === expected) {
			this.unlocked = true;
			return true;
		}
		return false;
	}
}

export const auth = new AuthState();
