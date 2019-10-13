'use strict';

const fs = require('fs');
const util = require('util');
const path = require('path');
const Parser = require('../lib/parsing.js');

const Fixtures = {
  DEFINITELY_TYPED: readFixtureSync('definitely-typed-types'),
};

describe('Parser', function() {
  it('should not hang when parsing tests/fixtures/*', function() {
    this.timeout(50 * 1000);
    Object.keys(Fixtures).forEach(function(fixtureName) {
      Fixtures[fixtureName].forEach(function(fixture) {
        if (fixture.skip) return;

        try {
          Parser.parse(fixture.typeExprStr);
        } catch (e) {
          if (e.name !== 'SyntaxError') {
            throw e;
          }
        }
      });
    });
  });
});


function readFixtureSync(fileName) {
  const filePath = path.resolve(__dirname, 'fixtures', fileName);

  return fs.readFileSync(filePath, 'utf8')
    .trim()
    .split(/\n/)
    .map(function(line, lineIdx) {
      return {
        // When the line starts with "//", we should skip it.
        skip: /^\/\//.test(line),

        typeExprStr: line.trim().replace(/^\{(.*)\}$/, '$1'),
        position: {
          filePath: filePath,
          lineno: lineIdx + 1,
        },
      };
    });
}
