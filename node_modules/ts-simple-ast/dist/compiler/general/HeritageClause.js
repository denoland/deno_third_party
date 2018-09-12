"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var tslib_1 = require("tslib");
var manipulation_1 = require("../../manipulation");
var common_1 = require("../common");
var HeritageClause = /** @class */ (function (_super) {
    tslib_1.__extends(HeritageClause, _super);
    function HeritageClause() {
        return _super !== null && _super.apply(this, arguments) || this;
    }
    /**
     * Gets all the type nodes for the heritage clause.
     */
    HeritageClause.prototype.getTypeNodes = function () {
        var _this = this;
        if (this.compilerNode.types == null)
            return [];
        return this.compilerNode.types.map(function (t) { return _this.getNodeFromCompilerNode(t); });
    };
    /**
     * Gets the heritage clause token.
     */
    HeritageClause.prototype.getToken = function () {
        return this.compilerNode.token;
    };
    HeritageClause.prototype.removeExpression = function (expressionNodeOrIndex) {
        var expressions = this.getTypeNodes();
        var expressionNodeToRemove = typeof expressionNodeOrIndex === "number" ? getExpressionFromIndex(expressionNodeOrIndex) : expressionNodeOrIndex;
        if (expressions.length === 1) {
            var heritageClauses = this.getParentSyntaxListOrThrow().getChildren();
            if (heritageClauses.length === 1)
                manipulation_1.removeChildren({ children: [heritageClauses[0].getParentSyntaxListOrThrow()], removePrecedingSpaces: true });
            else
                manipulation_1.removeChildren({ children: [this], removePrecedingSpaces: true });
        }
        else
            manipulation_1.removeCommaSeparatedChild(expressionNodeToRemove);
        return this;
        function getExpressionFromIndex(index) {
            return expressions[manipulation_1.verifyAndGetIndex(index, expressions.length - 1)];
        }
    };
    return HeritageClause;
}(common_1.Node));
exports.HeritageClause = HeritageClause;
