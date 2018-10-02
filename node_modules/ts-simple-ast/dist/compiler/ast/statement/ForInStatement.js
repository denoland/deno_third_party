"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var IterationStatement_1 = require("./IterationStatement");
exports.ForInStatementBase = IterationStatement_1.IterationStatement;
var ForInStatement = /** @class */ (function (_super) {
    tslib_1.__extends(ForInStatement, _super);
    function ForInStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this for in statement's initializer.
     */
    ForInStatement.prototype.getInitializer = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.initializer);
    };
    /**
     * Gets this for in statement's expression.
     */
    ForInStatement.prototype.getExpression = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.expression);
    };
    return ForInStatement;
}(exports.ForInStatementBase));
exports.ForInStatement = ForInStatement;
