'use strict';

const {expect} = require('chai');
const jsdoctypeparser = require('../index.js');

describe('jsdoctypeparser', function() {
  const expectedTypeMap = {
    parse: 'function',
    JSDocTypeSyntaxError: 'function',
    publish: 'function',
    createDefaultPublisher: 'function',
    traverse: 'function',
    NodeType: 'object',
    SyntaxType: 'object',
  };

  Object.keys(expectedTypeMap).forEach(function(key) {
    const expectedType = expectedTypeMap[key];
    describe('.' + key, function() {
      it('should be exported', function() {
        expect(jsdoctypeparser[key]).to.be.a(expectedType);
      });
    });
  });
});
