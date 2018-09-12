"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var Statement_1 = require("./Statement");
exports.SwitchStatementBase = base_1.ChildOrderableNode(Statement_1.Statement);
var SwitchStatement = /** @class */ (function (_super) {
    tslib_1.__extends(SwitchStatement, _super);
    function SwitchStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this switch statement's expression.
     */
    SwitchStatement.prototype.getExpression = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.expression);
    };
    /**
     * Gets this switch statement's case block.
     */
    SwitchStatement.prototype.getCaseBlock = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.caseBlock);
    };
    /**
     * Gets the switch statement's case block's clauses.
     */
    SwitchStatement.prototype.getClauses = function () {
        // convenience method
        return this.getCaseBlock().getClauses();
    };
    /**
     * Removes the specified clause based on the provided index.
     * @param index - Index.
     */
    SwitchStatement.prototype.removeClause = function (index) {
        return this.getCaseBlock().removeClause(index);
    };
    /**
     * Removes the specified clauses based on the provided index range.
     * @param indexRange - Index range.
     */
    SwitchStatement.prototype.removeClauses = function (indexRange) {
        return this.getCaseBlock().removeClauses(indexRange);
    };
    return SwitchStatement;
}(exports.SwitchStatementBase));
exports.SwitchStatement = SwitchStatement;
