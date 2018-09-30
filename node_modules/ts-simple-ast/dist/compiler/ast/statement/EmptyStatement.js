"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Statement_1 = require("./Statement");
exports.EmptyStatementBase = Statement_1.Statement;
var EmptyStatement = /** @class */ (function (_super) {
    tslib_1.__extends(EmptyStatement, _super);
    function EmptyStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return EmptyStatement;
}(exports.EmptyStatementBase));
exports.EmptyStatement = EmptyStatement;
