"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var base_1 = require("../base");
var common_1 = require("../common");
var StatementedNode_1 = require("./StatementedNode");
exports.CaseClauseBase = base_1.ChildOrderableNode(base_1.TextInsertableNode(StatementedNode_1.StatementedNode(common_1.Node)));
var CaseClause = /** @class */ (function (_super) {
    tslib_1.__extends(CaseClause, _super);
    function CaseClause() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets this switch statement's expression.
     */
    CaseClause.prototype.getExpression = function () {
        return this.getNodeFromCompilerNode(this.compilerNode.expression);
    };
    /**
     * Removes this case clause.
     */
    CaseClause.prototype.remove = function () {
        manipulation_1.removeClausedNodeChild(this);
    };
    return CaseClause;
}(exports.CaseClauseBase));
exports.CaseClause = CaseClause;
