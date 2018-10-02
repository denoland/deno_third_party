"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var IterationStatement_1 = require("./IterationStatement");
exports.WhileStatementBase = IterationStatement_1.IterationStatement;
var WhileStatement = /** @class */ (function (_super) {
    tslib_1.__extends(WhileStatement, _super);
    function WhileStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this while statement's expression.
     */
    WhileStatement.prototype.getExpression = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.expression);
    };
    return WhileStatement;
}(exports.WhileStatementBase));
exports.WhileStatement = WhileStatement;
