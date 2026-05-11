const value = Number.parseInt(input, 10);
if (Number.isNaN(value)) {
  throw new Error("invalid number");
}
element.textContent = userInput;
