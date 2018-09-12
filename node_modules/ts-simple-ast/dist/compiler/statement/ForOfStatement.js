"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var IterationStatement_1 = require("./IterationStatement");
exports.ForOfStatementBase = base_1.AwaitableNode(IterationStatement_1.IterationStatement);
var ForOfStatement = /** @class */ (function (_super) {
    tslib_1.__extends(ForOfStatement, _super);
    function ForOfStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this for of statement's initializer.
     */
    ForOfStatement.prototype.getInitializer = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.initializer);
    };
    /**
     * Gets this for of statement's expression.
     */
    ForOfStatement.prototype.getExpression = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.expression);
    };
    return ForOfStatement;
}(exports.ForOfStatementBase));
exports.ForOfStatement = ForOfStatement;
