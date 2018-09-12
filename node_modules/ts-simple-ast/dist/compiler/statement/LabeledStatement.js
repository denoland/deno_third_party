"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var base_1 = require("../base");
var Statement_1 = require("./Statement");
exports.LabeledStatementBase = base_1.JSDocableNode(base_1.ChildOrderableNode(Statement_1.Statement));
var LabeledStatement = /** @class */ (function (_super) {
    tslib_1.__extends(LabeledStatement, _super);
    function LabeledStatement() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this labeled statement's label
     */
    LabeledStatement.prototype.getLabel = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.label);
    };
    /**
     * Gets this labeled statement's statement
     */
    LabeledStatement.prototype.getStatement = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.statement);
    };
    return LabeledStatement;
}(exports.LabeledStatementBase));
exports.LabeledStatement = LabeledStatement;
