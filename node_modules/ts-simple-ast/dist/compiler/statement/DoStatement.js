"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var IterationStatement_1 = require("./IterationStatement");
exports.DoStatementBase = IterationStatement_1.IterationStatement;
var DoStatement = /** @class */ (function (_super) {
    tslib_1.__extends(DoStatement, _super);
    function DoStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this do statement's expression.
     */
    DoStatement.prototype.getExpression = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.expression);
    };
    return DoStatement;
}(exports.DoStatementBase));
exports.DoStatement = DoStatement;
