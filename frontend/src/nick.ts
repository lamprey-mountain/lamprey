// TODO: funnier names

const vowels = ["a", "e", "i", "o", "u"];
const consonants = [
	"p",
	"t",
	"k",
	"b",
	"d",
	"g",
	"m",
	"n",
	"s",
	"h",
	"z",
	"r",
	"l",
	"m",
	"j",
	"w",
	"y",
];

function rand(arr: string[]) {
	return arr[Math.floor(Math.random() * arr.length)];
}

export function generateNickname() {
	let name = "";
	for (
		let i = 0, vowel = false;
		i < Math.random() * 8 + 4;
		i++, vowel = !vowel
	) {
		name += vowel ? rand(vowels) : rand(consonants);
	}
	return name;
}
