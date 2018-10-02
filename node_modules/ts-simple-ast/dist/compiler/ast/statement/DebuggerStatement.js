"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var Statement_1 = require("./Statement");
exports.DebuggerStatementBase = Statement_1.Statement;
var DebuggerStatement = /** @class */ (function (_super) {
    tslib_1.__extends(DebuggerStatement, _super);
    function DebuggerStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    return DebuggerStatement;
}(exports.DebuggerStatementBase));
exports.DebuggerStatement = DebuggerStatement;
