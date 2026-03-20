	export default function metaphone(entry: string): string {
		const normalizedEntry = entry.trim().toLowerCase();
		const vowelArr = ['a', 'e', 'i', 'o', 'u'];
		let output = '';

		let entryWorker = '';
		const entryChars = Array.from(normalizedEntry);

		if (entryChars.length > 0) {
			entryWorker += entryChars[0];
		}
			for (let i = 0; i + 1 < entryChars.length; i += 1) {
			const p = entryChars[i];
			const c = entryChars[i + 1];
			if (p !== c || c === 'c') {
            entryWorker += c;
}
		}

		if (
			entryWorker.startsWith('kn')
			|| entryWorker.startsWith('gn')
			|| entryWorker.startsWith('pn')
			|| entryWorker.startsWith('ae')
			|| entryWorker.startsWith('wr')
		) {
			entryWorker = Array.from(entryWorker).slice(1).join('');
		}

		if (entryWorker.endsWith('mb')) {
			const workerChars = Array.from(entryWorker);
			workerChars.pop();
			entryWorker = workerChars.join('');
		}

		if (entryWorker.endsWith('gned')) {
			entryWorker = `${entryWorker.slice(0, -4)}ned`;
		} else if (entryWorker.endsWith('gn')) {
			entryWorker = `${entryWorker.slice(0, -2)}n`;
		} else if (entryWorker.endsWith('g')) {
			const workerChars = Array.from(entryWorker);
			workerChars.pop();
			entryWorker = workerChars.join('');
		}

		const charsArr = Array.from(entryWorker);
		let index = 0;

		while (index < charsArr.length) {
			switch (charsArr[index]) {
				case 's': {
					if (index + 2 < charsArr.length) {
						if (charsArr[index + 1] === 'c' && charsArr[index + 2] === 'h') {
							output += 'k';
							index += 3;
							continue;
						}

						if (
							charsArr[index + 1] === 'i'
							&& (charsArr[index + 2] === 'o' || charsArr[index + 2] === 'a')
						) {
							output += 'x';
							index += 3;
							continue;
						}
					}

					if (index + 1 < charsArr.length) {
						if (charsArr[index + 1] === 'h') {
							output += 'x';
							index += 2;
							continue;
						}
					}

					output += 's';
					index += 1;
					continue;
				}

				case 't': {
					if (index + 2 < charsArr.length) {
						if (
							charsArr[index + 1] === 'i'
							&& (charsArr[index + 2] === 'o' || charsArr[index + 2] === 'a')
						) {
							output += 'x';
							index += 3;
							continue;
						}

						if (charsArr[index + 1] === 'c' && charsArr[index + 2] === 'h') {
							index += 3;
							continue;
						}
					}

					if (index + 1 < charsArr.length) {
						if (charsArr[index + 1] === 'h') {
							output += '0';
							index += 2;
							continue;
						}
					}

					output += 't';
					index += 1;
					break;
				}

				case 'p': {
					if (index + 1 < charsArr.length) {
						if (charsArr[index + 1] === 'h') {
							output += 'f';
							index += 2;
							continue;
						}
					}

					output += 'p';
					index += 1;
					continue;
				}

				case 'k': {
                    output += 'k';
                    index += 1;
                    continue;
                }

				case 'c': {
					if (index + 2 < charsArr.length) {
						if (charsArr[index + 1] === 'i' && charsArr[index + 2] === 'a') {
							output += 'x';
							index += 3;
							continue;
						}
					}

					if (index < charsArr.length - 1) {
						switch (charsArr[index + 1]) {
                            case 'k': 
                            output+= 'k'
                            index += 2;
								continue;
							case 'h':
								output += 'x';
								index += 2;
								continue;
							case 'i':
							case 'e':
							case 'y':
								output += 's';
								index += 2;
								break;
							default:
								output += 'k';
								index += 1;
								break;
						}
					} else {
						output += charsArr[index];
						index += 1;
					}
					break;
				}

				case 'd': {
					if (index + 2 < charsArr.length) {
						if (
							charsArr[index + 1] === 'g'
							&& (
								charsArr[index + 2] === 'e'
								|| charsArr[index + 2] === 'y'
								|| charsArr[index + 2] === 'i'
							)
						) {
							output += 'j';
							index += 3;
							continue;
						}

						output += 't';
						index += 1;
						continue;
					}

					output += 't';
					index += 1;
					continue;
				}

				case 'g': {
					if (index + 1 < charsArr.length) {
						if (
							charsArr[index + 1] === 'h'
							&& index + 2 < charsArr.length
							&& !vowelArr.includes(charsArr[index + 2])
						) {
							index += 1;
							continue;
						}

						if (
							(
								charsArr[index + 1] === 'i'
								|| charsArr[index + 1] === 'e'
								|| charsArr[index + 1] === 'y'
							)
							&& (index === 0 || charsArr[index - 1] !== 'g')
						) {
							output += 'j';
							index += 2;
							continue;
						}

						output += 'k';
						index += 1;
						continue;
					}

					output += 'k';
					index += 1;
					continue;
				}

				case 'h': {
					if (index > 0 && index + 1 < charsArr.length) {
						if (
							vowelArr.includes(charsArr[index - 1])
							&& !vowelArr.includes(charsArr[index + 1])
						) {
							index += 1;
							continue;
						}

						output += 'h';
						index += 1;
						continue;
					}

					index += 1;
					continue;
				}

				case 'q':
					output += 'k';
					index += 1;
					continue;

				case 'v':
					output += 'f';
					index += 1;
					continue;

				case 'w': {
					if (index === 0 && index + 1 < charsArr.length && charsArr[index + 1] === 'h') {
						output += 'w';
						index += 2;
						continue;
					}

					if (index + 1 < charsArr.length) {
						if (vowelArr.includes(charsArr[index + 1])) {
							output += 'w';
							index += 1;
							continue;
						}

						index += 1;
						continue;
					}

					index += 1;
					continue;
				}

				case 'x': {
					if (index === 0) {
						output += 's';
						index += 1;
						continue;
					}

					output += 'ks';
					index += 1;
					continue;
				}

				case 'y': {
					if (index + 1 < charsArr.length) {
						if (vowelArr.includes(charsArr[index + 1])) {
							output += 'y';
							index += 1;
							continue;
						}

						index += 1;
						continue;
					}

					index += 1;
					continue;
				}

				case 'z':
					output += 's';
					index += 1;
					continue;

				case 'a':
				case 'e':
				case 'i':
				case 'o':
				case 'u': {
					if (index === 0) {
						output += charsArr[index];
						index += 1;
						continue;
					}

					index += 1;
					continue;
				}

				default:
                    output += charsArr[index];
					index += 1;
			}
		}

		return output;
	}

