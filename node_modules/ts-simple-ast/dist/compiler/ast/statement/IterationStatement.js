"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var Statement_1 = require("./Statement");
exports.IterationStatementBase = base_1.ChildOrderableNode(Statement_1.Statement);
var IterationStatement = /** @class */ (function (_super) {
    tslib_1.__extends(IterationStatement, _super);
    function IterationStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this iteration statement's statement.
     */
    IterationStatement.prototype.getStatement = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.statement);
    };
    return IterationStatement;
}(exports.IterationStatementBase));
exports.IterationStatement = IterationStatement;
