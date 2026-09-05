const DATATYPE_NAME = /^[A-Za-z_][A-Za-z0-9_]*$/u;
const NUMBER = /^[+-]?(?:(?:[0-9]+(?:\.[0-9]*)?)|(?:\.[0-9]+))(?:[eE][+-]?[0-9]+)?$/u;

/**
 * Decode the compact datatype descriptor used by telex.aes into the
 * transport-neutral AES datatype fields.
 */
export function parseDatatypeDescriptor(input, options = {}) {
  if (typeof input !== 'string') {
    throw new TypeError('Datatype descriptor must be a string');
  }
  const parser = new DatatypeParser(input, options);
  const descriptor = parser.parseDescriptor(0);
  parser.skipWhitespace();
  if (!parser.atEnd()) parser.fail('Unexpected trailing datatype syntax');
  return descriptor;
}

/**
 * Encode the transport-neutral AES datatype fields as one canonical Telex
 * datatype descriptor.
 */
export function formatDatatypeDescriptor(descriptor, options = {}) {
  assertDatatypeDescriptor(descriptor, options);
  return formatCheckedDatatypeDescriptor(descriptor);
}

function formatCheckedDatatypeDescriptor(descriptor) {
  const generics = descriptor.generics.length === 0
    ? ''
    : `<${descriptor.generics.map(formatGenericArgument).join(', ')}>`;
  const clarifiers = descriptor.clarifiers.length === 0
    ? ''
    : `[${descriptor.clarifiers.map(formatClarifier).join(', ')}]`;
  return `${descriptor.datatype}${generics}${clarifiers}`;
}

export function assertDatatypeDescriptor(descriptor, options = {}) {
  const limits = datatypeLimits(options);
  assertDatatypeDescriptorAtDepth(descriptor, limits, { items: 0 }, 0);
}

function assertDatatypeDescriptorAtDepth(descriptor, limits, state, depth) {
  if (descriptor === null || typeof descriptor !== 'object' || Array.isArray(descriptor)) {
    throw new TypeError('Datatype descriptor must be an object');
  }
  if (typeof descriptor.datatype !== 'string' || !DATATYPE_NAME.test(descriptor.datatype)) {
    throw new TypeError('Datatype must be an ASCII identifier');
  }
  if (!hasExactKeys(descriptor, ['datatype', 'generics', 'clarifiers'])) {
    throw new TypeError('Datatype descriptor has unknown or missing fields');
  }
  if (!Array.isArray(descriptor.generics)) {
    throw new TypeError('Datatype generics must be an array');
  }
  if (!Array.isArray(descriptor.clarifiers)) {
    throw new TypeError('Datatype clarifiers must be an array');
  }
  countLogicalItem(state, limits);
  if (descriptor.generics.length > 0 && depth > limits.maxDepth) {
    throw datatypeValidationError(
      'Datatype generic depth exceeds configured limit',
      'AES_DATATYPE_DEPTH',
    );
  }
  for (const argument of descriptor.generics) {
    assertGenericArgument(argument, limits, state, depth);
  }
  for (const clarifier of descriptor.clarifiers) {
    countLogicalItem(state, limits);
    assertTaggedLiteral(clarifier, 'clarifier');
  }
}

function assertGenericArgument(argument, limits, state, depth) {
  if (argument !== null
    && typeof argument === 'object'
    && !Array.isArray(argument)
    && Object.hasOwn(argument, 'datatype')) {
    assertDatatypeDescriptorAtDepth(argument, limits, state, depth + 1);
    return;
  }
  countLogicalItem(state, limits);
  assertTaggedLiteral(argument, 'generic argument', new Set(['NumberLiteral']));
}

function assertTaggedLiteral(literal, label, allowedKinds = new Set(['StringLiteral', 'NumberLiteral'])) {
  if (literal === null || typeof literal !== 'object' || Array.isArray(literal)) {
    throw new TypeError(`Datatype ${label} must be a tagged literal`);
  }
  if (!hasExactKeys(literal, ['kind', 'value'])) {
    throw new TypeError(`Datatype ${label} has unknown or missing fields`);
  }
  if (!allowedKinds.has(literal.kind) || typeof literal.value !== 'string') {
    throw new TypeError(`Invalid datatype ${label}`);
  }
  if (!hasOnlyUnicodeScalars(literal.value)) {
    throw new TypeError(`Datatype ${label} must contain only Unicode scalar values`);
  }
  if (literal.kind === 'NumberLiteral' && !NUMBER.test(literal.value)) {
    throw new TypeError(`Invalid numeric datatype ${label}`);
  }
}

function formatGenericArgument(argument) {
  return Object.hasOwn(argument, 'datatype')
    ? formatCheckedDatatypeDescriptor(argument)
    : formatNumberLiteral(argument, 'generic argument');
}

function formatClarifier(clarifier) {
  if (clarifier.kind === 'StringLiteral') return JSON.stringify(clarifier.value);
  return formatNumberLiteral(clarifier, 'clarifier');
}

function formatNumberLiteral(literal, label) {
  assertTaggedLiteral(literal, label, new Set(['NumberLiteral']));
  return literal.value;
}

class DatatypeParser {
  constructor(input, options) {
    this.input = input;
    this.cursor = 0;
    const limits = datatypeLimits(options);
    this.maxDepth = limits.maxDepth;
    this.maxItems = limits.maxItems;
    this.items = 0;
  }

  parseDescriptor(depth) {
    this.countItem();
    this.skipWhitespace();
    const datatype = this.parseName();
    this.skipWhitespace();
    if (this.peek() === '<' && depth > this.maxDepth) {
      this.fail('Datatype generic depth exceeds configured limit', 'TELEX_DATATYPE_LIMIT');
    }
    const generics = this.peek() === '<' ? this.parseGenerics(depth) : [];
    this.skipWhitespace();
    const clarifiers = this.peek() === '[' ? this.parseClarifiers() : [];
    return { datatype, generics, clarifiers };
  }

  parseName() {
    const match = this.input.slice(this.cursor).match(/^[A-Za-z_][A-Za-z0-9_]*/u);
    if (match === null) this.fail('Expected datatype name');
    this.cursor += match[0].length;
    return match[0];
  }

  parseGenerics(depth) {
    this.consume('<');
    this.skipWhitespace();
    if (this.peek() === '>') this.fail('Generic argument list must not be empty');
    const values = [];
    while (true) {
      values.push(this.parseGenericArgument(depth));
      this.skipWhitespace();
      if (this.peek() === '>') {
        this.cursor += 1;
        return values;
      }
      this.consume(',');
      this.skipWhitespace();
    }
  }

  parseGenericArgument(depth) {
    this.skipWhitespace();
    if (/[A-Za-z_]/u.test(this.peek() ?? '')) return this.parseDescriptor(depth + 1);
    this.countItem();
    return { kind: 'NumberLiteral', value: this.parseNumber(',>') };
  }

  parseClarifiers() {
    this.consume('[');
    this.skipWhitespace();
    if (this.peek() === ']') this.fail('Clarifier list must not be empty');
    const values = [];
    while (true) {
      this.countItem();
      values.push(this.peek() === '"'
        ? { kind: 'StringLiteral', value: this.parseString() }
        : { kind: 'NumberLiteral', value: this.parseNumber(',]') });
      this.skipWhitespace();
      if (this.peek() === ']') {
        this.cursor += 1;
        return values;
      }
      this.consume(',');
      this.skipWhitespace();
    }
  }

  parseString() {
    const start = this.cursor;
    this.cursor += 1;
    let escaped = false;
    while (!this.atEnd()) {
      const character = this.input[this.cursor];
      this.cursor += 1;
      if (escaped) {
        escaped = false;
      } else if (character === '\\') {
        escaped = true;
      } else if (character === '"') {
        const encoded = this.input.slice(start, this.cursor);
        try {
          const value = JSON.parse(encoded);
          if (typeof value !== 'string') this.fail('Expected string clarifier');
          if (!hasOnlyUnicodeScalars(value)) this.fail('Invalid Unicode scalar in string clarifier');
          return value;
        } catch {
          this.fail('Invalid string clarifier');
        }
      }
    }
    this.fail('Unterminated string clarifier');
  }

  parseNumber(delimiters) {
    const start = this.cursor;
    while (!this.atEnd() && !delimiters.includes(this.peek()) && !/\s/u.test(this.peek())) {
      this.cursor += 1;
    }
    const value = this.input.slice(start, this.cursor);
    if (!NUMBER.test(value)) this.fail('Expected numeric datatype argument');
    return value;
  }

  consume(expected) {
    this.skipWhitespace();
    if (this.peek() !== expected) this.fail(`Expected '${expected}'`);
    this.cursor += 1;
  }

  skipWhitespace() {
    while (!this.atEnd() && /[ \t\r\n]/u.test(this.peek())) this.cursor += 1;
  }

  peek() {
    return this.input[this.cursor];
  }

  atEnd() {
    return this.cursor >= this.input.length;
  }

  countItem() {
    this.items += 1;
    if (this.items > this.maxItems) {
      this.fail('Datatype component count exceeds configured limit', 'TELEX_DATATYPE_LIMIT');
    }
  }

  fail(message, code = 'TELEX_INVALID_DATATYPE') {
    const error = new TypeError(`${message} at datatype offset ${this.cursor}`);
    error.code = code;
    throw error;
  }
}

function hasOnlyUnicodeScalars(value) {
  for (const character of value) {
    const scalar = character.codePointAt(0);
    if (scalar >= 0xd800 && scalar <= 0xdfff) return false;
  }
  return true;
}

function hasExactKeys(value, expected) {
  const keys = Object.keys(value);
  return keys.length === expected.length && expected.every((key) => Object.hasOwn(value, key));
}

function datatypeLimits(options) {
  const maxDepth = options.maxDepth ?? 1;
  const maxItems = options.maxItems ?? 4096;
  if (!Number.isSafeInteger(maxDepth) || maxDepth < 0) {
    throw new TypeError('Datatype maxDepth must be a non-negative safe integer');
  }
  if (!Number.isSafeInteger(maxItems) || maxItems < 1) {
    throw new TypeError('Datatype maxItems must be a positive safe integer');
  }
  return { maxDepth, maxItems };
}

function countLogicalItem(state, limits) {
  state.items += 1;
  if (state.items > limits.maxItems) {
    throw datatypeValidationError(
      'Datatype component count exceeds configured limit',
      'AES_DATATYPE_LIMIT',
    );
  }
}

function datatypeValidationError(message, code) {
  const error = new TypeError(message);
  error.code = code;
  return error;
}
