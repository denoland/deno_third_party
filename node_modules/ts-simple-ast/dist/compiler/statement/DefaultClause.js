"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var base_1 = require("../base");
var common_1 = require("../common");
var StatementedNode_1 = require("./StatementedNode");
exports.DefaultClauseBase = base_1.ChildOrderableNode(base_1.TextInsertableNode(StatementedNode_1.StatementedNode(common_1.Node)));
var DefaultClause = /** @class */ (function (_super) {
    tslib_1.__extends(DefaultClause, _super);
    function DefaultClause() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Removes the default clause.
     */
    DefaultClause.prototype.remove = function () {
        manipulation_1.removeClausedNodeChild(this);
    };
    return DefaultClause;
}(exports.DefaultClauseBase));
exports.DefaultClause = DefaultClause;
