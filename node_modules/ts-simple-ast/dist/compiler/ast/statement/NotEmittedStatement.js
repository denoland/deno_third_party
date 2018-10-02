"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Statement_1 = require("./Statement");
exports.NotEmittedStatementBase = Statement_1.Statement;
var NotEmittedStatement = /** @class */ (function (_super) {
    tslib_1.__extends(NotEmittedStatement, _super);
    function NotEmittedStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return NotEmittedStatement;
}(exports.NotEmittedStatementBase));
exports.NotEmittedStatement = NotEmittedStatement;
