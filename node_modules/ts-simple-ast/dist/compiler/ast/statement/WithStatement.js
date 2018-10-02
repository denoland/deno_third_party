"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var Statement_1 = require("./Statement");
exports.WithStatementBase = base_1.ChildOrderableNode(Statement_1.Statement);
var WithStatement = /** @class */ (function (_super) {
    tslib_1.__extends(WithStatement, _super);
    function WithStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this with statement's expression.
     */
    WithStatement.prototype.getExpression = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.expression);
    };
    /**
     * Gets this with statement's statement.
     */
    WithStatement.prototype.getStatement = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.statement);
    };
    return WithStatement;
}(exports.WithStatementBase));
exports.WithStatement = WithStatement;
