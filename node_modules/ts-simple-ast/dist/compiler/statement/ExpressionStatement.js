"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var Statement_1 = require("./Statement");
exports.ExpressionStatementBase = base_1.JSDocableNode(base_1.ChildOrderableNode(Statement_1.Statement));
var ExpressionStatement = /** @class */ (function (_super) {
    tslib_1.__extends(ExpressionStatement, _super);
    function ExpressionStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this expression statement's expression.
     */
    ExpressionStatement.prototype.getExpression = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.expression);
    };
    return ExpressionStatement;
}(exports.ExpressionStatementBase));
exports.ExpressionStatement = ExpressionStatement;
